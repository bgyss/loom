// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event types for the multi-agent system.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
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
	/// Task dependency was resolved
	DependencyResolved(DependencyResolvedEvent),
}

/// Event emitted when a task dependency is resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolvedEvent {
	/// Task that had the dependency
	pub task_id: TaskId,
	/// Dependency that was resolved
	pub dependency_id: TaskId,
	/// Whether all dependencies are now resolved
	pub all_resolved: bool,
	/// Timestamp of resolution
	pub timestamp: DateTime<Utc>,
}

/// Event type enumeration for filtering and persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventType {
	Created,
	Assigned,
	Started,
	Progress,
	Completed,
	Failed,
	Cancelled,
	NeedsIteration,
	DependencyResolved,
}

impl TaskEventType {
	#[must_use]
	pub fn as_str(&self) -> &'static str {
		match self {
			TaskEventType::Created => "created",
			TaskEventType::Assigned => "assigned",
			TaskEventType::Started => "started",
			TaskEventType::Progress => "progress",
			TaskEventType::Completed => "completed",
			TaskEventType::Failed => "failed",
			TaskEventType::Cancelled => "cancelled",
			TaskEventType::NeedsIteration => "needs_iteration",
			TaskEventType::DependencyResolved => "dependency_resolved",
		}
	}
}

impl std::fmt::Display for TaskEventType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

/// Event priority for back-pressure handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
	Low,
	Normal,
	High,
	Critical,
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
			TaskEvent::DependencyResolved(e) => &e.task_id,
		}
	}

	/// Get the event type.
	#[must_use]
	pub fn event_type(&self) -> TaskEventType {
		match self {
			TaskEvent::Created { .. } => TaskEventType::Created,
			TaskEvent::Assigned { .. } => TaskEventType::Assigned,
			TaskEvent::Started { .. } => TaskEventType::Started,
			TaskEvent::Progress { .. } => TaskEventType::Progress,
			TaskEvent::Completed(_) => TaskEventType::Completed,
			TaskEvent::Failed { .. } => TaskEventType::Failed,
			TaskEvent::Cancelled { .. } => TaskEventType::Cancelled,
			TaskEvent::NeedsIteration { .. } => TaskEventType::NeedsIteration,
			TaskEvent::DependencyResolved(_) => TaskEventType::DependencyResolved,
		}
	}

	/// Get the planner ID associated with this event, if any.
	#[must_use]
	pub fn planner_id(&self) -> Option<&PlannerId> {
		match self {
			TaskEvent::Created { planner_id, .. } => Some(planner_id),
			TaskEvent::Completed(completion) => Some(&completion.context_snapshot.planner_id),
			_ => None,
		}
	}

	/// Get the worker ID associated with this event, if any.
	#[must_use]
	pub fn worker_id(&self) -> Option<&WorkerId> {
		match self {
			TaskEvent::Assigned { worker_id, .. } => Some(worker_id),
			TaskEvent::Started { worker_id, .. } => Some(worker_id),
			TaskEvent::Progress { worker_id, .. } => Some(worker_id),
			TaskEvent::Completed(completion) => Some(&completion.worker_id),
			TaskEvent::Failed { worker_id, .. } => Some(worker_id),
			_ => None,
		}
	}

	/// Get the timestamp of this event.
	#[must_use]
	pub fn timestamp(&self) -> DateTime<Utc> {
		match self {
			TaskEvent::Completed(completion) => completion.completed_at,
			TaskEvent::DependencyResolved(e) => e.timestamp,
			_ => Utc::now(),
		}
	}

	/// Get the priority of this event for back-pressure handling.
	#[must_use]
	pub fn priority(&self) -> EventPriority {
		match self {
			TaskEvent::Completed(_) => EventPriority::High,
			TaskEvent::Failed { .. } => EventPriority::High,
			TaskEvent::DependencyResolved(_) => EventPriority::High,
			TaskEvent::Created { .. } => EventPriority::Normal,
			TaskEvent::NeedsIteration { .. } => EventPriority::Normal,
			TaskEvent::Started { .. } => EventPriority::Normal,
			TaskEvent::Assigned { .. } => EventPriority::Normal,
			TaskEvent::Progress { .. } => EventPriority::Low,
			TaskEvent::Cancelled { .. } => EventPriority::Low,
		}
	}
}

/// Context provided when a planner wakes up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeUpContext {
	/// The event that triggered wake-up
	pub trigger_event: TaskEvent,
	/// Time since planner last planned (in seconds)
	pub time_since_last_plan_secs: u64,
	/// Number of tasks completed since last planning
	pub tasks_completed_since_plan: u32,
	/// Pending tasks from this planner
	pub pending_tasks: Vec<TaskId>,
}

impl WakeUpContext {
	pub fn new(
		trigger_event: TaskEvent,
		time_since_last_plan: Duration,
		tasks_completed_since_plan: u32,
		pending_tasks: Vec<TaskId>,
	) -> Self {
		Self {
			trigger_event,
			time_since_last_plan_secs: time_since_last_plan.as_secs(),
			tasks_completed_since_plan,
			pending_tasks,
		}
	}

	#[must_use]
	pub fn time_since_last_plan(&self) -> Duration {
		Duration::from_secs(self.time_since_last_plan_secs)
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
	fn test_task_event_type() {
		let event = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		assert_eq!(event.event_type(), TaskEventType::Created);
		assert_eq!(event.event_type().as_str(), "created");
	}

	#[test]
	fn test_dependency_resolved_event() {
		let task_id = TaskId::new();
		let dep_id = TaskId::new();
		let event = TaskEvent::DependencyResolved(DependencyResolvedEvent {
			task_id: task_id.clone(),
			dependency_id: dep_id.clone(),
			all_resolved: true,
			timestamp: Utc::now(),
		});
		assert_eq!(event.task_id(), &task_id);
		assert_eq!(event.event_type(), TaskEventType::DependencyResolved);
		assert_eq!(event.priority(), EventPriority::High);
	}

	#[test]
	fn test_event_priority() {
		let completed_event = TaskEvent::Progress {
			task_id: TaskId::new(),
			worker_id: WorkerId::new(),
			percentage: 50,
			current_step: "test".to_string(),
		};
		assert_eq!(completed_event.priority(), EventPriority::Low);

		let failed_event = TaskEvent::Failed {
			task_id: TaskId::new(),
			worker_id: WorkerId::new(),
			error: "test error".to_string(),
		};
		assert_eq!(failed_event.priority(), EventPriority::High);
	}

	#[test]
	fn test_wake_up_context() {
		let trigger = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		let ctx = WakeUpContext::new(trigger, Duration::from_secs(60), 5, vec![]);
		assert_eq!(ctx.time_since_last_plan(), Duration::from_secs(60));
		assert_eq!(ctx.tasks_completed_since_plan, 5);
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
