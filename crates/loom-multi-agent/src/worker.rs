// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Worker - executes tasks assigned by planners.

use std::sync::Arc;

use chrono::Utc;
use tokio::sync::mpsc;
use tracing::{info, instrument, warn};

use loom_multi_agent_core::{Task, TaskCompletion, TaskOutput, WorkerId};
use loom_multi_agent_events::EventBus;
use loom_server_multi_agent::{MultiAgentRepository, TaskQueue};

use crate::Result;

/// A worker that executes tasks.
pub struct Worker<R: MultiAgentRepository> {
	/// Worker identity
	id: WorkerId,
	/// Task queue
	task_queue: Arc<TaskQueue<R>>,
	/// Event bus
	event_bus: Arc<EventBus>,
	/// Shutdown receiver
	shutdown_rx: mpsc::Receiver<()>,
}

impl<R: MultiAgentRepository + 'static> Worker<R> {
	/// Create a new worker.
	pub fn new(
		id: WorkerId,
		task_queue: Arc<TaskQueue<R>>,
		event_bus: Arc<EventBus>,
		shutdown_rx: mpsc::Receiver<()>,
	) -> Self {
		Self {
			id,
			task_queue,
			event_bus,
			shutdown_rx,
		}
	}

	/// Get the worker ID.
	pub fn id(&self) -> &WorkerId {
		&self.id
	}

	/// Run the main worker loop.
	#[instrument(skip(self), fields(worker_id = %self.id))]
	pub async fn run(self) -> Result<()> {
		info!("Worker {} starting", self.id);

		let _ = self.event_bus.publish(loom_multi_agent_events::TaskEvent::Created {
			task_id: loom_multi_agent_core::TaskId::new(),
			planner_id: loom_multi_agent_core::PlannerId::new(),
			description: format!("Worker {} started", self.id),
		});

		// Move shutdown_rx out of self to avoid borrow conflicts in select!
		let id = self.id;
		let task_queue = self.task_queue;
		let mut shutdown_rx = self.shutdown_rx;

		loop {
			tokio::select! {
				_ = shutdown_rx.recv() => {
					info!("Worker {} received shutdown signal", id);
					break;
				}
				task = task_queue.dequeue(&id) => {
					match task {
						Ok(Some(task)) => {
							let task_id = task.id.clone();
							match Self::execute_task(&task_queue, &id, task).await {
								Ok(completion) => {
									if let Err(e) = task_queue.complete(completion).await {
										warn!(%task_id, "Failed to complete task: {}", e);
									}
								}
								Err(e) => {
									warn!(%task_id, "Task execution failed: {}", e);
									let _ = task_queue.fail(&task_id, &id, e.to_string()).await;
								}
							}
						}
						Ok(None) => {
							tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
						}
						Err(e) => {
							warn!("Failed to dequeue task: {}", e);
							tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
						}
					}
				}
			}
		}

		info!("Worker {} shutting down", id);
		Ok(())
	}

	/// Execute a task.
	#[instrument(skip(task_queue, task), fields(task_id = %task.id))]
	async fn execute_task(
		task_queue: &TaskQueue<R>,
		worker_id: &WorkerId,
		task: Task,
	) -> Result<TaskCompletion> {
		let start_time = Utc::now();
		info!("Executing task: {}", task.description);

		task_queue.start_task(&task.id, worker_id).await?;

		task_queue
			.update_progress(&task.id, worker_id, 10, "Analyzing task".to_string())
			.await?;

		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		task_queue
			.update_progress(&task.id, worker_id, 50, "Working on implementation".to_string())
			.await?;

		tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

		task_queue
			.update_progress(&task.id, worker_id, 90, "Finishing up".to_string())
			.await?;

		let end_time = Utc::now();
		let duration_ms = (end_time - start_time).num_milliseconds() as u64;

		let completion = TaskCompletion {
			task_id: task.id.clone(),
			worker_id: worker_id.clone(),
			completed_at: end_time,
			duration_ms,
			output: TaskOutput {
				summary: format!("Completed: {}", task.description),
				notes: vec!["Task completed successfully".to_string()],
				artifacts: Vec::new(),
				suggested_tasks: Vec::new(),
				learnings: Vec::new(),
			},
			changes: Vec::new(),
			commit_sha: None,
			verifications: Vec::new(),
			context_snapshot: Default::default(),
			unblocked_tasks: Vec::new(),
		};

		info!("Task {} completed in {}ms", task.id, duration_ms);
		Ok(completion)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_worker_types() {
		let id = WorkerId::new();
		assert!(id.to_string().starts_with("worker-"));
	}
}
