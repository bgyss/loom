// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Orchestrator - the singleton coordinator for the multi-agent system.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{info, instrument, warn};

use loom_multi_agent_core::{
	CheckpointId, Domain, DomainId, PlannerId, ProjectConfig, ProjectMetrics, ProjectPhase,
	ProjectRunId, ProjectState, ProjectStatus,
};
use loom_multi_agent_events::{EventBus, PlannerEvent};
use loom_server_multi_agent::{MultiAgentRepository, TaskQueue};

use crate::planner::DomainPlanner;
use crate::worker_pool::WorkerPool;
use crate::Result;

/// The singleton orchestrator that coordinates all agents.
pub struct Orchestrator<R: MultiAgentRepository> {
	/// Project state
	project_state: Arc<RwLock<ProjectState>>,
	/// Active planners by domain
	planners: HashMap<DomainId, PlannerHandle>,
	/// Task queue
	task_queue: Arc<TaskQueue<R>>,
	/// Worker pool
	worker_pool: WorkerPool<R>,
	/// Event bus
	event_bus: Arc<EventBus>,
	/// Repository
	repository: Arc<R>,
	/// Configuration
	config: ProjectConfig,
	/// Shutdown signal
	shutdown_tx: mpsc::Sender<()>,
	shutdown_rx: mpsc::Receiver<()>,
}

/// Handle to a running planner.
#[allow(dead_code)]
struct PlannerHandle {
	planner_id: PlannerId,
	event_tx: mpsc::Sender<PlannerEvent>,
	task_handle: tokio::task::JoinHandle<()>,
}

impl<R: MultiAgentRepository + 'static> Orchestrator<R> {
	/// Create a new orchestrator.
	pub fn new(
		repository: Arc<R>,
		event_bus: Arc<EventBus>,
		org_id: String,
		project_path: String,
		goal: String,
		created_by: String,
		config: ProjectConfig,
	) -> Self {
		let project_state = ProjectState::new(org_id, project_path, goal, created_by);
		let project_run_id = project_state.id.clone();

		let task_queue = Arc::new(TaskQueue::new(
			repository.clone(),
			event_bus.clone(),
			project_run_id.clone(),
		));

		let worker_pool = WorkerPool::new(
			repository.clone(),
			task_queue.clone(),
			event_bus.clone(),
			project_run_id,
			config.max_workers,
		);

		let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

		Self {
			project_state: Arc::new(RwLock::new(project_state)),
			planners: HashMap::new(),
			task_queue,
			worker_pool,
			event_bus,
			repository,
			config,
			shutdown_tx,
			shutdown_rx,
		}
	}

	/// Get the project run ID.
	pub async fn project_run_id(&self) -> ProjectRunId {
		self.project_state.read().await.id.clone()
	}

	/// Initialize the orchestrator.
	#[instrument(skip(self))]
	pub async fn initialize(&mut self) -> Result<()> {
		let state = self.project_state.read().await;
		self.repository.create_project_run(&state).await?;
		drop(state);

		info!("Orchestrator initialized");
		Ok(())
	}

	/// Analyze the project and spawn domain planners.
	#[instrument(skip(self))]
	pub async fn spawn_planners(&mut self) -> Result<Vec<DomainId>> {
		let state = self.project_state.read().await;
		let project_run_id = state.id.clone();
		drop(state);

		let domains = self.analyze_project_domains().await?;
		let mut domain_ids = Vec::new();

		for domain in domains {
			let domain_id = domain.id.clone();
			self.repository.create_domain(&project_run_id, &domain).await?;

			let planner_id = PlannerId::new();
			self.repository
				.create_planner(&planner_id, &domain_id, None)
				.await?;

			let (event_tx, event_rx) = mpsc::channel(100);

			let planner = DomainPlanner::new(
				planner_id.clone(),
				domain.clone(),
				event_rx,
				self.task_queue.clone(),
			);

			let planner_id_for_spawn = planner_id.clone();
			let task_handle = tokio::spawn(async move {
				if let Err(e) = planner.run().await {
					warn!("Planner {} exited with error: {}", planner_id_for_spawn, e);
				}
			});

			self.planners.insert(
				domain_id.clone(),
				PlannerHandle {
					planner_id,
					event_tx,
					task_handle,
				},
			);

			domain_ids.push(domain_id);
		}

		{
			let mut state = self.project_state.write().await;
			state.phase = ProjectPhase::Development { iteration: 1 };
			self.repository.update_project_run(&state).await?;
		}

		info!("Spawned {} domain planners", domain_ids.len());
		Ok(domain_ids)
	}

	/// Analyze the project and identify domains.
	async fn analyze_project_domains(&self) -> Result<Vec<Domain>> {
		let state = self.project_state.read().await;
		let _project_path = &state.project_path;

		let domains = vec![
			Domain::new(
				"backend".to_string(),
				vec!["crates/**/*.rs".to_string()],
			),
			Domain::new(
				"frontend".to_string(),
				vec!["web/**/*.ts".to_string(), "web/**/*.svelte".to_string()],
			),
			Domain::new(
				"infrastructure".to_string(),
				vec!["infra/**/*".to_string(), "*.nix".to_string()],
			),
		];

		Ok(domains)
	}

	/// Start the worker pool.
	#[instrument(skip(self))]
	pub async fn start_workers(&mut self, count: u32) -> Result<()> {
		self.worker_pool.spawn_workers(count).await?;
		info!("Started {} workers", count);
		Ok(())
	}

	/// Run the main orchestration loop.
	#[instrument(skip(self))]
	pub async fn run(&mut self) -> Result<()> {
		info!("Orchestrator starting main loop");

		let mut subscription = self.event_bus.subscribe();

		loop {
			tokio::select! {
				_ = self.shutdown_rx.recv() => {
					info!("Shutdown signal received");
					break;
				}
				event = subscription.recv() => {
					match event {
						Ok(task_event) => {
							if let Err(e) = self.handle_task_event(task_event).await {
								warn!("Error handling task event: {}", e);
							}
						}
						Err(e) => {
							warn!("Event subscription error: {}", e);
						}
					}
				}
			}
		}

		self.shutdown().await?;
		Ok(())
	}

	/// Handle a task event.
	async fn handle_task_event(&mut self, event: loom_multi_agent_events::TaskEvent) -> Result<()> {
		use loom_multi_agent_events::TaskEvent;

		match event {
			TaskEvent::Completed(completion) => {
				let task = self.repository.get_task(&completion.task_id).await?;
				if let Some(task) = task {
					if let Some(handle) = self.planners.get(&self.get_domain_for_planner(&task.created_by).await?) {
						let _ = handle
							.event_tx
							.send(PlannerEvent::TaskCompleted(completion))
							.await;
					}
				}

				let mut state = self.project_state.write().await;
				state.metrics.tasks_completed += 1;
				self.repository.update_project_run(&state).await?;
			}
			TaskEvent::Failed { task_id, error, .. } => {
				warn!(%task_id, %error, "Task failed");

				let mut state = self.project_state.write().await;
				state.metrics.tasks_failed += 1;
				self.repository.update_project_run(&state).await?;
			}
			TaskEvent::Created { .. } => {
				let mut state = self.project_state.write().await;
				state.metrics.tasks_created += 1;
			}
			_ => {}
		}

		Ok(())
	}

	/// Get the domain ID for a planner.
	async fn get_domain_for_planner(&self, planner_id: &PlannerId) -> Result<DomainId> {
		self.repository
			.get_planner_domain(planner_id)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Planner not found"))
	}

	/// Create a checkpoint.
	#[instrument(skip(self))]
	pub async fn checkpoint(&self, reason: Option<&str>) -> Result<CheckpointId> {
		let state = self.project_state.read().await;
		let checkpoint_id = CheckpointId::new();

		let snapshot = serde_json::to_string(&*state)?;
		self.repository
			.create_checkpoint(&checkpoint_id, &state.id, &snapshot, reason)
			.await?;

		info!(%checkpoint_id, "Checkpoint created");
		Ok(checkpoint_id)
	}

	/// Pause the project.
	#[instrument(skip(self))]
	pub async fn pause(&mut self) -> Result<()> {
		let mut state = self.project_state.write().await;
		state.status = ProjectStatus::Paused;
		self.repository.update_project_run(&state).await?;

		for handle in self.planners.values() {
			let _ = handle.event_tx.send(PlannerEvent::Shutdown).await;
		}

		self.worker_pool.stop_all().await?;

		info!("Project paused");
		Ok(())
	}

	/// Resume the project.
	#[instrument(skip(self))]
	pub async fn resume(&mut self) -> Result<()> {
		let mut state = self.project_state.write().await;
		state.status = ProjectStatus::Running;
		self.repository.update_project_run(&state).await?;

		self.worker_pool.spawn_workers(self.config.max_workers).await?;

		info!("Project resumed");
		Ok(())
	}

	/// Shutdown the orchestrator.
	#[instrument(skip(self))]
	pub async fn shutdown(&mut self) -> Result<()> {
		info!("Shutting down orchestrator");

		for handle in self.planners.values() {
			let _ = handle.event_tx.send(PlannerEvent::Shutdown).await;
		}

		for (_, handle) in self.planners.drain() {
			let _ = handle.task_handle.await;
		}

		self.worker_pool.stop_all().await?;

		let mut state = self.project_state.write().await;
		state.status = ProjectStatus::Completed;
		state.completed_at = Some(chrono::Utc::now());
		self.repository.update_project_run(&state).await?;

		info!("Orchestrator shutdown complete");
		Ok(())
	}

	/// Get the current project state.
	pub async fn get_state(&self) -> ProjectState {
		self.project_state.read().await.clone()
	}

	/// Get the current metrics.
	pub async fn get_metrics(&self) -> ProjectMetrics {
		self.project_state.read().await.metrics.clone()
	}

	/// Request shutdown.
	pub fn request_shutdown(&self) {
		let tx = self.shutdown_tx.clone();
		tokio::spawn(async move {
			let _ = tx.send(()).await;
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_orchestrator_types() {
		let config = ProjectConfig::default();
		assert_eq!(config.max_workers, 10);
	}
}
