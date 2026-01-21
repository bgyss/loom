// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Task queue with priority and dependency resolution.

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::instrument;

use chrono::Utc;
use loom_multi_agent_core::{ProjectRunId, Task, TaskCompletion, TaskId, TaskStatus, WorkerId};
use loom_multi_agent_events::{DependencyResolvedEvent, EventBus, TaskEvent};

use crate::error::Result;
use crate::repository::MultiAgentRepository;

/// Task queue that manages task assignment with priority and dependency resolution.
pub struct TaskQueue<R: MultiAgentRepository> {
	/// Repository for persistence
	repository: Arc<R>,
	/// Event bus for notifications
	event_bus: Arc<EventBus>,
	/// Project run ID
	project_run_id: ProjectRunId,
	/// Lock for atomic operations
	lock: RwLock<()>,
}

impl<R: MultiAgentRepository> TaskQueue<R> {
	/// Create a new task queue.
	pub fn new(
		repository: Arc<R>,
		event_bus: Arc<EventBus>,
		project_run_id: ProjectRunId,
	) -> Self {
		Self {
			repository,
			event_bus,
			project_run_id,
			lock: RwLock::new(()),
		}
	}

	/// Enqueue a new task.
	#[instrument(skip(self, task), fields(task_id = %task.id))]
	pub async fn enqueue(&self, task: Task) -> Result<TaskId> {
		let _guard = self.lock.write().await;

		let task_id = task.id.clone();
		let planner_id = task.created_by.clone();
		let description = task.description.clone();

		self.repository.create_task(&task).await?;

		for dep in &task.dependencies {
			self.repository.add_task_dependency(&task_id, dep).await?;
		}

		let _ = self.event_bus.publish(TaskEvent::Created {
			task_id: task_id.clone(),
			planner_id,
			description,
		});

		tracing::info!(%task_id, "Task enqueued");
		Ok(task_id)
	}

	/// Dequeue the highest priority ready task for a worker.
	#[instrument(skip(self), fields(worker_id = %worker_id))]
	pub async fn dequeue(&self, worker_id: &WorkerId) -> Result<Option<Task>> {
		let _guard = self.lock.write().await;

		let ready_tasks = self.repository.get_ready_tasks(&self.project_run_id, 1).await?;

		if let Some(mut task) = ready_tasks.into_iter().next() {
			let now = chrono::Utc::now();
			task.status = TaskStatus::Assigned {
				worker_id: worker_id.clone(),
				assigned_at: now,
			};

			self.repository.update_task(&task).await?;
			self.repository.assign_task_to_worker(worker_id, &task.id).await?;

			let _ = self.event_bus.publish(TaskEvent::Assigned {
				task_id: task.id.clone(),
				worker_id: worker_id.clone(),
			});

			tracing::info!(task_id = %task.id, %worker_id, "Task dequeued");
			Ok(Some(task))
		} else {
			Ok(None)
		}
	}

	/// Mark a task as started.
	#[instrument(skip(self), fields(task_id = %task_id, worker_id = %worker_id))]
	pub async fn start_task(&self, task_id: &TaskId, worker_id: &WorkerId) -> Result<()> {
		let _guard = self.lock.write().await;

		let now = chrono::Utc::now();
		let status = TaskStatus::InProgress {
			worker_id: worker_id.clone(),
			started_at: now,
			progress: Default::default(),
		};

		self.repository.update_task_status(task_id, &status).await?;

		let _ = self.event_bus.publish(TaskEvent::Started {
			task_id: task_id.clone(),
			worker_id: worker_id.clone(),
		});

		tracing::info!(%task_id, %worker_id, "Task started");
		Ok(())
	}

	/// Update task progress.
	#[instrument(skip(self), fields(task_id = %task_id))]
	pub async fn update_progress(
		&self,
		task_id: &TaskId,
		worker_id: &WorkerId,
		percentage: u8,
		current_step: String,
	) -> Result<()> {
		let _ = self.event_bus.publish(TaskEvent::Progress {
			task_id: task_id.clone(),
			worker_id: worker_id.clone(),
			percentage,
			current_step,
		});

		Ok(())
	}

	/// Complete a task and notify the planner.
	#[instrument(skip(self, completion), fields(task_id = %completion.task_id))]
	pub async fn complete(&self, mut completion: TaskCompletion) -> Result<()> {
		let _guard = self.lock.write().await;

		let task_id = completion.task_id.clone();
		let worker_id = completion.worker_id.clone();

		let now = Utc::now();
		let status = TaskStatus::Completed {
			completed_at: now,
			summary: completion.output.summary.clone(),
		};

		self.repository.update_task_status(&task_id, &status).await?;
		self.repository.create_completion(&completion).await?;
		self.repository.clear_worker_task(&worker_id).await?;

		let dependent_tasks = self.repository.get_dependent_tasks(&task_id).await?;
		let mut unblocked_tasks = Vec::new();

		for dep_task_id in &dependent_tasks {
			self.repository
				.remove_task_dependency(dep_task_id, &task_id)
				.await?;

			let remaining_deps = self.repository.get_task_dependencies(dep_task_id).await?;
			let all_resolved = remaining_deps.is_empty();

			let _ = self.event_bus.publish(TaskEvent::DependencyResolved(
				DependencyResolvedEvent {
					task_id: dep_task_id.clone(),
					dependency_id: task_id.clone(),
					all_resolved,
					timestamp: Utc::now(),
				},
			));

			if all_resolved {
				unblocked_tasks.push(dep_task_id.clone());
			}
		}

		completion.unblocked_tasks = unblocked_tasks.clone();

		let _ = self
			.event_bus
			.publish(TaskEvent::Completed(Box::new(completion)));

		tracing::info!(
			%task_id,
			unblocked = unblocked_tasks.len(),
			"Task completed, dependent tasks unblocked"
		);
		Ok(())
	}

	/// Mark a task as failed.
	#[instrument(skip(self), fields(task_id = %task_id))]
	pub async fn fail(&self, task_id: &TaskId, worker_id: &WorkerId, error: String) -> Result<()> {
		let _guard = self.lock.write().await;

		let now = chrono::Utc::now();
		let status = TaskStatus::Failed {
			failed_at: now,
			error: error.clone(),
		};

		self.repository.update_task_status(task_id, &status).await?;
		self.repository.clear_worker_task(worker_id).await?;

		let _ = self.event_bus.publish(TaskEvent::Failed {
			task_id: task_id.clone(),
			worker_id: worker_id.clone(),
			error,
		});

		tracing::warn!(%task_id, "Task failed");
		Ok(())
	}

	/// Mark a task as needing iteration.
	#[instrument(skip(self), fields(task_id = %task_id))]
	pub async fn needs_iteration(
		&self,
		task_id: &TaskId,
		feedback: String,
		iteration: u32,
	) -> Result<()> {
		let _guard = self.lock.write().await;

		let status = TaskStatus::NeedsIteration {
			feedback: feedback.clone(),
			iteration,
		};

		self.repository.update_task_status(task_id, &status).await?;

		let _ = self.event_bus.publish(TaskEvent::NeedsIteration {
			task_id: task_id.clone(),
			feedback,
			iteration,
		});

		tracing::info!(%task_id, iteration, "Task needs iteration");
		Ok(())
	}

	/// Cancel a task.
	#[instrument(skip(self), fields(task_id = %task_id))]
	pub async fn cancel(&self, task_id: &TaskId, reason: String) -> Result<()> {
		let _guard = self.lock.write().await;

		let now = chrono::Utc::now();
		let status = TaskStatus::Cancelled {
			cancelled_at: now,
			reason: reason.clone(),
		};

		self.repository.update_task_status(task_id, &status).await?;

		if let Some(task) = self.repository.get_task(task_id).await? {
			if let TaskStatus::Assigned { worker_id, .. } | TaskStatus::InProgress { worker_id, .. } =
				task.status
			{
				self.repository.clear_worker_task(&worker_id).await?;
			}
		}

		let _ = self.event_bus.publish(TaskEvent::Cancelled {
			task_id: task_id.clone(),
			reason,
		});

		tracing::info!(%task_id, "Task cancelled");
		Ok(())
	}

	/// Get queue statistics.
	pub async fn stats(&self) -> Result<QueueStats> {
		let _guard = self.lock.read().await;

		let pending = self
			.repository
			.list_tasks_by_status(&self.project_run_id, "pending")
			.await?
			.len();
		let in_progress = self
			.repository
			.list_tasks_by_status(&self.project_run_id, "in_progress")
			.await?
			.len();
		let completed = self
			.repository
			.list_tasks_by_status(&self.project_run_id, "completed")
			.await?
			.len();
		let failed = self
			.repository
			.list_tasks_by_status(&self.project_run_id, "failed")
			.await?
			.len();

		Ok(QueueStats {
			pending: pending as u32,
			in_progress: in_progress as u32,
			completed: completed as u32,
			failed: failed as u32,
		})
	}
}

/// Statistics about the task queue.
#[derive(Debug, Clone)]
pub struct QueueStats {
	/// Number of pending tasks
	pub pending: u32,
	/// Number of in-progress tasks
	pub in_progress: u32,
	/// Number of completed tasks
	pub completed: u32,
	/// Number of failed tasks
	pub failed: u32,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_queue_stats_default() {
		let stats = QueueStats {
			pending: 5,
			in_progress: 2,
			completed: 10,
			failed: 1,
		};
		assert_eq!(stats.pending, 5);
		assert_eq!(stats.in_progress, 2);
	}
}
