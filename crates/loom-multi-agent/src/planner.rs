// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Domain planner - creates and manages tasks for a domain.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{info, instrument, warn};

use loom_multi_agent_core::{
	Domain, PlannerId, Task, TaskCompletion, TaskId, TaskSpecification, TaskType,
};
use loom_multi_agent_events::{PlannerEvent, TaskEvent, WakeUpContext};
use loom_server_multi_agent::{MultiAgentRepository, TaskQueue};

use crate::Result;

/// Metrics for planner activity.
#[derive(Debug, Default)]
pub struct PlannerMetrics {
	/// Number of times the planner woke up
	pub wake_ups: AtomicU64,
	/// Tasks completed since last planning session
	pub tasks_completed_since_plan: AtomicU32,
	/// Tasks created by this planner
	pub tasks_created: AtomicU64,
}

/// A domain planner that creates and manages tasks for a specific domain.
pub struct DomainPlanner<R: MultiAgentRepository> {
	/// Planner identity
	id: PlannerId,
	/// Domain this planner is responsible for
	domain: Domain,
	/// Event receiver
	event_rx: mpsc::Receiver<PlannerEvent>,
	/// Task queue
	task_queue: Arc<TaskQueue<R>>,
	/// Time of last planning session
	last_plan_time: Instant,
	/// Pending tasks from this planner
	pending_tasks: Vec<TaskId>,
	/// Metrics
	metrics: PlannerMetrics,
}

impl<R: MultiAgentRepository + 'static> DomainPlanner<R> {
	/// Create a new domain planner.
	pub fn new(
		id: PlannerId,
		domain: Domain,
		event_rx: mpsc::Receiver<PlannerEvent>,
		task_queue: Arc<TaskQueue<R>>,
	) -> Self {
		Self {
			id,
			domain,
			event_rx,
			task_queue,
			last_plan_time: Instant::now(),
			pending_tasks: Vec::new(),
			metrics: PlannerMetrics::default(),
		}
	}

	/// Get the planner metrics.
	pub fn metrics(&self) -> &PlannerMetrics {
		&self.metrics
	}

	/// Get the planner ID.
	pub fn id(&self) -> &PlannerId {
		&self.id
	}

	/// Get the domain.
	pub fn domain(&self) -> &Domain {
		&self.domain
	}

	/// Run the main planner loop.
	#[instrument(skip(self), fields(planner_id = %self.id, domain = %self.domain.name))]
	pub async fn run(mut self) -> Result<()> {
		info!("Domain planner starting for {}", self.domain.name);

		let initial_tasks = self.explore_and_plan().await?;
		for task in initial_tasks {
			let task_id = task.id.clone();
			self.task_queue.enqueue(task).await?;
			self.pending_tasks.push(task_id);
			self.metrics.tasks_created.fetch_add(1, Ordering::Relaxed);
		}

		loop {
			match self.event_rx.recv().await {
				Some(PlannerEvent::TaskCompleted(completion)) => {
					self.metrics.wake_ups.fetch_add(1, Ordering::Relaxed);
					self.metrics
						.tasks_completed_since_plan
						.fetch_add(1, Ordering::Relaxed);

					let wake_up_context = self.capture_wake_up_context(&TaskEvent::Completed(
						Box::new((*completion).clone()),
					));

					info!(
						task_id = %completion.task_id,
						time_since_last_plan_secs = wake_up_context.time_since_last_plan_secs,
						tasks_completed = wake_up_context.tasks_completed_since_plan,
						"Planner woke up: task completed"
					);

					self.pending_tasks.retain(|t| t != &completion.task_id);

					let new_tasks = self.plan_next_steps(*completion).await?;
					for task in new_tasks {
						let task_id = task.id.clone();
						self.task_queue.enqueue(task).await?;
						self.pending_tasks.push(task_id);
						self.metrics.tasks_created.fetch_add(1, Ordering::Relaxed);
					}

					self.last_plan_time = Instant::now();
					self.metrics
						.tasks_completed_since_plan
						.store(0, Ordering::Relaxed);
				}
				Some(PlannerEvent::IterationFeedback { task_id, feedback }) => {
					self.metrics.wake_ups.fetch_add(1, Ordering::Relaxed);
					info!(%task_id, "Planner woke up: iteration feedback");

					let iteration_task =
						self.create_iteration_task(&task_id.to_string(), &feedback).await?;
					let task_id = iteration_task.id.clone();
					self.task_queue.enqueue(iteration_task).await?;
					self.pending_tasks.push(task_id);
					self.metrics.tasks_created.fetch_add(1, Ordering::Relaxed);
				}
				Some(PlannerEvent::DomainChanged { changes, .. }) => {
					self.metrics.wake_ups.fetch_add(1, Ordering::Relaxed);
					info!("Planner woke up: domain changed, {} files affected", changes.len());

					let tasks = self.react_to_changes(&changes).await?;
					for task in tasks {
						let task_id = task.id.clone();
						self.task_queue.enqueue(task).await?;
						self.pending_tasks.push(task_id);
						self.metrics.tasks_created.fetch_add(1, Ordering::Relaxed);
					}
				}
				Some(PlannerEvent::PlanningRequested { reason }) => {
					self.metrics.wake_ups.fetch_add(1, Ordering::Relaxed);
					info!("Planner woke up: planning requested - {}", reason);

					let tasks = self.plan_for_reason(&reason).await?;
					for task in tasks {
						let task_id = task.id.clone();
						self.task_queue.enqueue(task).await?;
						self.pending_tasks.push(task_id);
						self.metrics.tasks_created.fetch_add(1, Ordering::Relaxed);
					}
				}
				Some(PlannerEvent::Shutdown) => {
					info!("Shutdown requested");
					break;
				}
				Some(_) => {}
				None => {
					warn!("Event channel closed");
					break;
				}
			}
		}

		info!("Domain planner {} shutting down", self.id);
		Ok(())
	}

	/// Capture context when waking up.
	fn capture_wake_up_context(&self, trigger: &TaskEvent) -> WakeUpContext {
		WakeUpContext::new(
			trigger.clone(),
			self.last_plan_time.elapsed(),
			self.metrics.tasks_completed_since_plan.load(Ordering::Relaxed),
			self.pending_tasks.clone(),
		)
	}

	/// Explore the domain and generate initial tasks.
	#[instrument(skip(self))]
	async fn explore_and_plan(&self) -> Result<Vec<Task>> {
		info!("Exploring domain {} and generating initial tasks", self.domain.name);

		let exploration_task = Task::new(
			self.id.clone(),
			TaskType::Exploration {
				questions: vec![
					format!("What are the main components in {}?", self.domain.name),
					"What patterns are used?".to_string(),
					"What are the key dependencies?".to_string(),
				],
			},
			format!("Explore {} domain structure", self.domain.name),
			TaskSpecification {
				requirements: vec![
					"Identify main components".to_string(),
					"Document patterns used".to_string(),
					"List key dependencies".to_string(),
				],
				..Default::default()
			},
		);

		Ok(vec![exploration_task])
	}

	/// Plan next steps after task completion.
	#[instrument(skip(self, completion), fields(task_id = %completion.task_id))]
	async fn plan_next_steps(&self, completion: TaskCompletion) -> Result<Vec<Task>> {
		let mut new_tasks = Vec::new();

		self.absorb_learnings(&completion).await?;

		for suggestion in &completion.output.suggested_tasks {
			if self.should_accept_suggestion(suggestion) {
				let task = self.create_task_from_suggestion(suggestion);
				new_tasks.push(task);
			}
		}

		let follow_ups = self.analyze_changes_for_follow_ups(&completion).await?;
		new_tasks.extend(follow_ups);

		info!(
			task_id = %completion.task_id,
			new_tasks = new_tasks.len(),
			"Planned next steps"
		);

		Ok(new_tasks)
	}

	/// Absorb learnings from a task completion.
	#[instrument(skip(self, completion))]
	async fn absorb_learnings(&self, completion: &TaskCompletion) -> Result<()> {
		for learning in &completion.output.learnings {
			info!(
				learning_type = %learning.learning_type,
				"Absorbed learning: {}",
				learning.content
			);
		}
		Ok(())
	}

	/// Check if we should accept a worker's suggestion.
	fn should_accept_suggestion(
		&self,
		suggestion: &loom_multi_agent_core::TaskSuggestion,
	) -> bool {
		suggestion.priority >= 0
	}

	/// Create a task from a worker suggestion.
	fn create_task_from_suggestion(
		&self,
		suggestion: &loom_multi_agent_core::TaskSuggestion,
	) -> Task {
		Task::new(
			self.id.clone(),
			TaskType::Feature {
				acceptance_criteria: vec![suggestion.description.clone()],
			},
			suggestion.description.clone(),
			TaskSpecification {
				requirements: vec![suggestion.description.clone()],
				hints: vec![suggestion.rationale.clone()],
				..Default::default()
			},
		)
	}

	/// Analyze file changes to determine follow-up tasks.
	#[instrument(skip(self, completion))]
	async fn analyze_changes_for_follow_ups(
		&self,
		completion: &TaskCompletion,
	) -> Result<Vec<Task>> {
		let mut tasks = Vec::new();

		let needs_tests = completion.changes.iter().any(|c| {
			c.path.extension().is_some_and(|e| e == "rs")
				&& !c.path.to_string_lossy().contains("test")
		});

		if needs_tests && completion.output.suggested_tasks.is_empty() {
			let changed_files: Vec<_> = completion
				.changes
				.iter()
				.map(|c| c.path.display().to_string())
				.collect();

			let task = Task::new(
				self.id.clone(),
				TaskType::Test {
					coverage_target: Some(0.8),
				},
				format!("Add tests for changed files: {}", changed_files.join(", ")),
				TaskSpecification {
					requirements: vec!["Ensure adequate test coverage".to_string()],
					context: completion
						.changes
						.iter()
						.map(|c| loom_multi_agent_core::ContextItem {
							context_type: "changed_file".to_string(),
							content: c.path.display().to_string(),
							source_file: Some(c.path.clone()),
						})
						.collect(),
					..Default::default()
				},
			);
			tasks.push(task);
		}

		Ok(tasks)
	}

	/// Create an iteration task based on feedback.
	#[instrument(skip(self))]
	async fn create_iteration_task(&self, original_task_id: &str, feedback: &str) -> Result<Task> {
		let task = Task::new(
			self.id.clone(),
			TaskType::BugFix {
				reproduction_steps: vec!["See original task".to_string()],
				expected_behavior: feedback.to_string(),
			},
			format!("Iteration for task {}: {}", original_task_id, feedback),
			TaskSpecification {
				requirements: vec![feedback.to_string()],
				hints: vec![format!("Original task: {}", original_task_id)],
				..Default::default()
			},
		);

		Ok(task)
	}

	/// React to domain changes.
	#[instrument(skip(self, changes))]
	async fn react_to_changes(&self, changes: &[std::path::PathBuf]) -> Result<Vec<Task>> {
		let mut tasks = Vec::new();

		if changes.iter().any(|p| p.extension().is_some_and(|e| e == "rs")) {
			let task = Task::new(
				self.id.clone(),
				TaskType::Test {
					coverage_target: Some(0.8),
				},
				"Update tests for changed files".to_string(),
				TaskSpecification {
					requirements: vec!["Ensure tests pass".to_string()],
					context: changes
						.iter()
						.map(|p| loom_multi_agent_core::ContextItem {
							context_type: "changed_file".to_string(),
							content: p.display().to_string(),
							source_file: Some(p.clone()),
						})
						.collect(),
					..Default::default()
				},
			);
			tasks.push(task);
		}

		Ok(tasks)
	}

	/// Plan tasks for a specific reason.
	#[instrument(skip(self))]
	async fn plan_for_reason(&self, reason: &str) -> Result<Vec<Task>> {
		let task = Task::new(
			self.id.clone(),
			TaskType::Exploration {
				questions: vec![format!("How to address: {}", reason)],
			},
			format!("Investigation: {}", reason),
			TaskSpecification {
				requirements: vec![reason.to_string()],
				..Default::default()
			},
		);

		Ok(vec![task])
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_planner_types() {
		let domain = Domain::new("test".to_string(), vec!["*.rs".to_string()]);
		assert_eq!(domain.name, "test");
	}
}
