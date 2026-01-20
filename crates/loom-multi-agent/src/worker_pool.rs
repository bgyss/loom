// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Worker pool - manages worker lifecycle.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, instrument, warn};

use loom_multi_agent_core::{ProjectRunId, WorkerId};
use loom_multi_agent_events::EventBus;
use loom_server_multi_agent::{MultiAgentRepository, TaskQueue};

use crate::worker::Worker;
use crate::Result;

/// Handle to a running worker.
struct WorkerHandle {
	worker_id: WorkerId,
	shutdown_tx: mpsc::Sender<()>,
	task_handle: tokio::task::JoinHandle<()>,
}

/// Manages a pool of workers.
pub struct WorkerPool<R: MultiAgentRepository> {
	/// Repository
	repository: Arc<R>,
	/// Task queue
	task_queue: Arc<TaskQueue<R>>,
	/// Event bus
	event_bus: Arc<EventBus>,
	/// Project run ID
	project_run_id: ProjectRunId,
	/// Maximum workers
	max_workers: u32,
	/// Active workers
	workers: HashMap<WorkerId, WorkerHandle>,
}

impl<R: MultiAgentRepository + 'static> WorkerPool<R> {
	/// Create a new worker pool.
	pub fn new(
		repository: Arc<R>,
		task_queue: Arc<TaskQueue<R>>,
		event_bus: Arc<EventBus>,
		project_run_id: ProjectRunId,
		max_workers: u32,
	) -> Self {
		Self {
			repository,
			task_queue,
			event_bus,
			project_run_id,
			max_workers,
			workers: HashMap::new(),
		}
	}

	/// Spawn workers up to the specified count.
	#[instrument(skip(self), fields(project_run_id = %self.project_run_id))]
	pub async fn spawn_workers(&mut self, count: u32) -> Result<Vec<WorkerId>> {
		let count = count.min(self.max_workers - self.workers.len() as u32);
		let mut spawned = Vec::new();

		for _ in 0..count {
			let worker_id = self.spawn_worker().await?;
			spawned.push(worker_id);
		}

		info!("Spawned {} workers, total: {}", count, self.workers.len());
		Ok(spawned)
	}

	/// Spawn a single worker.
	async fn spawn_worker(&mut self) -> Result<WorkerId> {
		let worker_id = WorkerId::new();
		let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

		self.repository
			.create_worker(&worker_id, &self.project_run_id)
			.await?;

		let worker = Worker::new(
			worker_id.clone(),
			self.task_queue.clone(),
			self.event_bus.clone(),
			shutdown_rx,
		);

		let wid = worker_id.clone();
		let task_handle = tokio::spawn(async move {
			if let Err(e) = worker.run().await {
				warn!("Worker {} exited with error: {}", wid, e);
			}
		});

		self.workers.insert(
			worker_id.clone(),
			WorkerHandle {
				worker_id: worker_id.clone(),
				shutdown_tx,
				task_handle,
			},
		);

		info!("Spawned worker {}", worker_id);
		Ok(worker_id)
	}

	/// Stop a specific worker.
	#[instrument(skip(self), fields(worker_id = %worker_id))]
	pub async fn stop_worker(&mut self, worker_id: &WorkerId) -> Result<()> {
		if let Some(handle) = self.workers.remove(worker_id) {
			let _ = handle.shutdown_tx.send(()).await;
			let _ = handle.task_handle.await;

			self.repository.update_worker_status(worker_id, "stopped").await?;
			info!("Stopped worker {}", worker_id);
		}

		Ok(())
	}

	/// Stop all workers.
	#[instrument(skip(self))]
	pub async fn stop_all(&mut self) -> Result<()> {
		let worker_ids: Vec<_> = self.workers.keys().cloned().collect();

		for worker_id in worker_ids {
			if let Some(handle) = self.workers.remove(&worker_id) {
				let _ = handle.shutdown_tx.send(()).await;
			}
		}

		for (_, handle) in self.workers.drain() {
			let _ = handle.task_handle.await;
		}

		info!("Stopped all workers");
		Ok(())
	}

	/// Get the number of active workers.
	pub fn active_count(&self) -> usize {
		self.workers.len()
	}

	/// Get the list of active worker IDs.
	pub fn active_workers(&self) -> Vec<WorkerId> {
		self.workers.keys().cloned().collect()
	}

	/// Check if a worker is active.
	pub fn is_active(&self, worker_id: &WorkerId) -> bool {
		self.workers.contains_key(worker_id)
	}

	/// Scale to a specific number of workers.
	#[instrument(skip(self))]
	pub async fn scale_to(&mut self, target: u32) -> Result<()> {
		let current = self.workers.len() as u32;

		if target > current {
			let to_spawn = target - current;
			self.spawn_workers(to_spawn).await?;
		} else if target < current {
			let to_stop = current - target;
			let workers_to_stop: Vec<_> = self
				.workers
				.keys()
				.take(to_stop as usize)
				.cloned()
				.collect();

			for worker_id in workers_to_stop {
				self.stop_worker(&worker_id).await?;
			}
		}

		info!("Scaled to {} workers", self.workers.len());
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_worker_pool_types() {
		let worker_id = WorkerId::new();
		assert!(worker_id.to_string().starts_with("worker-"));
	}
}
