// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event types for the multi-agent system.

use std::path::PathBuf;

use loom_multi_agent_core::{
	CheckpointId, DomainId, PlannerId, ProjectRunId, TaskCompletion, TaskId, WorkerId,
};
use serde::{Deserialize, Serialize};

/// Events related to task lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
	/// Task was created
	Created {
		task_id: TaskId,
		planner_id: PlannerId,
		description: String,
	},
	/// Task was assigned to a worker
	Assigned {
		task_id: TaskId,
		worker_id: WorkerId,
	},
	/// Task execution started
	Started {
		task_id: TaskId,
		worker_id: WorkerId,
	},
	/// Task progress updated
	Progress {
		task_id: TaskId,
		worker_id: WorkerId,
		percentage: u8,
		current_step: String,
	},
	/// Task completed
	Completed(Box<TaskCompletion>),
	/// Task failed
	Failed {
		task_id: TaskId,
		worker_id: WorkerId,
		error: String,
	},
	/// Task cancelled
	Cancelled { task_id: TaskId, reason: String },
	/// Task needs iteration
	NeedsIteration {
		task_id: TaskId,
		feedback: String,
		iteration: u32,
	},
}

impl TaskEvent {
	/// Get the task ID associated with this event.
	#[must_use]
	pub fn task_id(&self) -> &TaskId {
		match self {
			TaskEvent::Created { task_id, .. } => task_id,
			TaskEvent::Assigned { task_id, .. } => task_id,
			TaskEvent::Started { task_id, .. } => task_id,
			TaskEvent::Progress { task_id, .. } => task_id,
			TaskEvent::Completed(completion) => &completion.task_id,
			TaskEvent::Failed { task_id, .. } => task_id,
			TaskEvent::Cancelled { task_id, .. } => task_id,
			TaskEvent::NeedsIteration { task_id, .. } => task_id,
		}
	}
}

/// Events that wake up domain planners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlannerEvent {
	/// A task assigned by this planner completed
	TaskCompleted(Box<TaskCompletion>),
	/// A dependent domain changed
	DomainChanged {
		domain_id: DomainId,
		changes: Vec<PathBuf>,
	},
	/// Orchestrator requested planning
	PlanningRequested { reason: String },
	/// Resource became available
	ResourceAvailable { resource_type: String },
	/// Checkpoint restored (planner should re-orient)
	CheckpointRestored { checkpoint_id: CheckpointId },
	/// Sub-planner completed its scope
	SubPlannerCompleted {
		planner_id: PlannerId,
		summary: String,
	},
	/// Iteration feedback from judge
	IterationFeedback { task_id: TaskId, feedback: String },
	/// Shutdown requested
	Shutdown,
}

impl PlannerEvent {
	/// Check if this is a shutdown event.
	#[must_use]
	pub fn is_shutdown(&self) -> bool {
		matches!(self, PlannerEvent::Shutdown)
	}

	/// Check if this event requires immediate attention.
	#[must_use]
	pub fn is_urgent(&self) -> bool {
		matches!(
			self,
			PlannerEvent::Shutdown | PlannerEvent::CheckpointRestored { .. }
		)
	}
}

/// Events from workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkerEvent {
	/// Worker started
	Started { worker_id: WorkerId },
	/// Worker picked up a task
	TaskPickedUp {
		worker_id: WorkerId,
		task_id: TaskId,
	},
	/// Worker made progress
	Progress {
		worker_id: WorkerId,
		task_id: TaskId,
		percentage: u8,
		step: String,
	},
	/// Worker completed a task
	TaskCompleted {
		worker_id: WorkerId,
		task_id: TaskId,
		summary: String,
	},
	/// Worker failed a task
	TaskFailed {
		worker_id: WorkerId,
		task_id: TaskId,
		error: String,
	},
	/// Worker is idle
	Idle { worker_id: WorkerId },
	/// Worker stopped
	Stopped { worker_id: WorkerId, reason: String },
}

impl WorkerEvent {
	/// Get the worker ID associated with this event.
	#[must_use]
	pub fn worker_id(&self) -> &WorkerId {
		match self {
			WorkerEvent::Started { worker_id } => worker_id,
			WorkerEvent::TaskPickedUp { worker_id, .. } => worker_id,
			WorkerEvent::Progress { worker_id, .. } => worker_id,
			WorkerEvent::TaskCompleted { worker_id, .. } => worker_id,
			WorkerEvent::TaskFailed { worker_id, .. } => worker_id,
			WorkerEvent::Idle { worker_id } => worker_id,
			WorkerEvent::Stopped { worker_id, .. } => worker_id,
		}
	}
}

/// Events for the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorEvent {
	/// Project run started
	ProjectStarted { run_id: ProjectRunId },
	/// Project run completed
	ProjectCompleted { run_id: ProjectRunId },
	/// Project run failed
	ProjectFailed { run_id: ProjectRunId, error: String },
	/// Planner spawned
	PlannerSpawned {
		planner_id: PlannerId,
		domain_id: DomainId,
	},
	/// Planner stopped
	PlannerStopped {
		planner_id: PlannerId,
		reason: String,
	},
	/// Worker spawned
	WorkerSpawned {
		worker_id: WorkerId,
		run_id: ProjectRunId,
	},
	/// Worker stopped
	WorkerStopped {
		worker_id: WorkerId,
		reason: String,
	},
	/// Checkpoint created
	CheckpointCreated {
		checkpoint_id: CheckpointId,
		run_id: ProjectRunId,
	},
	/// Health alert
	HealthAlert { run_id: ProjectRunId, message: String },
	/// Shutdown requested
	Shutdown,
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_multi_agent_core::TaskId;

	#[test]
	fn test_task_event_task_id() {
		let task_id = TaskId::new();
		let event = TaskEvent::Created {
			task_id: task_id.clone(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		assert_eq!(event.task_id(), &task_id);
	}

	#[test]
	fn test_planner_event_is_shutdown() {
		assert!(PlannerEvent::Shutdown.is_shutdown());
		assert!(!PlannerEvent::PlanningRequested {
			reason: "test".to_string()
		}
		.is_shutdown());
	}

	#[test]
	fn test_worker_event_worker_id() {
		let worker_id = WorkerId::new();
		let event = WorkerEvent::Started {
			worker_id: worker_id.clone(),
		};
		assert_eq!(event.worker_id(), &worker_id);
	}
}
