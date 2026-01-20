// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Domain planner - creates and manages tasks for a domain.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, instrument, warn};

use loom_multi_agent_core::{
	Domain, PlannerId, Task, TaskCompletion, TaskSpecification, TaskType,
};
use loom_multi_agent_events::PlannerEvent;
use loom_server_multi_agent::{MultiAgentRepository, TaskQueue};

use crate::Result;

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
		}
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
			self.task_queue.enqueue(task).await?;
		}

		loop {
			match self.event_rx.recv().await {
				Some(PlannerEvent::TaskCompleted(completion)) => {
					info!(task_id = %completion.task_id, "Task completed, planning next steps");
					let new_tasks = self.on_task_completed(*completion).await?;
					for task in new_tasks {
						self.task_queue.enqueue(task).await?;
					}
				}
				Some(PlannerEvent::IterationFeedback { task_id, feedback }) => {
					info!(%task_id, "Received iteration feedback");
					let iteration_task = self.create_iteration_task(&task_id.to_string(), &feedback).await?;
					self.task_queue.enqueue(iteration_task).await?;
				}
				Some(PlannerEvent::DomainChanged { changes, .. }) => {
					info!("Domain changed, {} files affected", changes.len());
					let tasks = self.react_to_changes(&changes).await?;
					for task in tasks {
						self.task_queue.enqueue(task).await?;
					}
				}
				Some(PlannerEvent::PlanningRequested { reason }) => {
					info!("Planning requested: {}", reason);
					let tasks = self.plan_for_reason(&reason).await?;
					for task in tasks {
						self.task_queue.enqueue(task).await?;
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

	/// Handle task completion and plan next steps.
	#[instrument(skip(self, completion), fields(task_id = %completion.task_id))]
	async fn on_task_completed(&self, completion: TaskCompletion) -> Result<Vec<Task>> {
		let mut new_tasks = Vec::new();

		for suggestion in &completion.output.suggested_tasks {
			let task = Task::new(
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
			);
			new_tasks.push(task);
		}

		Ok(new_tasks)
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
