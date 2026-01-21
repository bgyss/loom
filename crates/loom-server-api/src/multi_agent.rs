// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! API types for multi-agent coordination system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Request to create a new multi-agent project run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateProjectRequest {
	/// Path to the project
	pub project_path: String,
	/// Goal for the project run
	pub goal: String,
	/// Optional configuration overrides
	pub config: Option<ProjectConfigRequest>,
}

/// Configuration for a project run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ProjectConfigRequest {
	/// Maximum number of concurrent workers
	pub max_workers: Option<u32>,
	/// Maximum number of concurrent planners
	pub max_planners: Option<u32>,
	/// Whether to auto-commit changes
	pub auto_commit: Option<bool>,
	/// Maximum iterations per task
	pub max_iterations: Option<u32>,
}

/// Response for a project run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ProjectRunResponse {
	pub id: String,
	pub org_id: String,
	pub project_path: String,
	pub goal: String,
	pub phase: String,
	pub status: String,
	pub metrics: ProjectMetricsResponse,
	pub started_at: DateTime<Utc>,
	pub completed_at: Option<DateTime<Utc>>,
	pub created_by: String,
}

/// Metrics for a project run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ProjectMetricsResponse {
	pub tasks_created: u64,
	pub tasks_completed: u64,
	pub tasks_failed: u64,
	pub tasks_cancelled: u64,
	pub lines_added: u64,
	pub lines_removed: u64,
	pub test_pass_rate: f64,
	pub avg_task_duration_ms: u64,
	pub active_workers: u32,
	pub active_planners: u32,
}

/// Response for listing project runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListProjectRunsResponse {
	pub projects: Vec<ProjectRunResponse>,
}

/// Response for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct TaskResponse {
	pub id: String,
	pub parent_id: Option<String>,
	pub planner_id: String,
	pub priority: i32,
	pub task_type: String,
	pub description: String,
	pub status: String,
	pub iteration: u32,
	pub affected_files: Vec<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

/// Response for listing tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListTasksResponse {
	pub tasks: Vec<TaskResponse>,
}

/// Query parameters for listing tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListTasksQuery {
	/// Filter by status
	pub status: Option<String>,
	/// Maximum number of tasks to return
	pub limit: Option<u32>,
	/// Offset for pagination
	pub offset: Option<u32>,
}

/// Response for a worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WorkerResponse {
	pub id: String,
	pub status: String,
	pub current_task_id: Option<String>,
	pub created_at: DateTime<Utc>,
	pub last_active_at: Option<DateTime<Utc>>,
}

/// Response for listing workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListWorkersResponse {
	pub workers: Vec<WorkerResponse>,
}

/// Request to create a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateCheckpointRequest {
	/// Optional reason for the checkpoint
	pub reason: Option<String>,
}

/// Response for a checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CheckpointResponse {
	pub id: String,
	pub project_run_id: String,
	pub created_at: DateTime<Utc>,
	pub reason: Option<String>,
}

/// SSE event for project updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ProjectEvent {
	/// Task was created
	TaskCreated {
		task_id: String,
		description: String,
	},
	/// Task completed
	TaskCompleted {
		task_id: String,
		summary: String,
	},
	/// Task failed
	TaskFailed {
		task_id: String,
		error: String,
	},
	/// Worker progress update
	WorkerProgress {
		worker_id: String,
		task_id: String,
		percentage: u8,
	},
	/// Planner woke up
	PlannerWake {
		planner_id: String,
		reason: String,
		new_tasks: u32,
	},
	/// Project phase changed
	PhaseChanged {
		old_phase: String,
		new_phase: String,
	},
	/// Project completed
	ProjectCompleted {
		summary: String,
	},
}

/// Success response for multi-agent operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct MultiAgentSuccessResponse {
	pub message: String,
}

/// Error response for multi-agent operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct MultiAgentErrorResponse {
	pub error: String,
	pub message: String,
}

/// Query parameters for SSE event stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EventStreamQuery {
	/// Filter by event types (comma-separated)
	pub filter: Option<String>,
	/// Replay events since this timestamp (ISO 8601)
	pub since: Option<DateTime<Utc>>,
}

/// Query parameters for event history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EventHistoryQuery {
	/// Filter by events after this time
	pub since: Option<DateTime<Utc>>,
	/// Filter by events before this time
	pub until: Option<DateTime<Utc>>,
	/// Filter by event types (comma-separated)
	pub event_types: Option<String>,
	/// Filter by planner ID
	pub planner_id: Option<String>,
	/// Filter by task ID
	pub task_id: Option<String>,
	/// Maximum number of events to return
	pub limit: Option<u32>,
	/// Offset for pagination
	pub offset: Option<u32>,
}

/// Response for event history.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EventHistoryResponse {
	pub events: Vec<TaskEventResponse>,
	pub total: u64,
	pub has_more: bool,
}

/// A task event in API response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct TaskEventResponse {
	pub id: String,
	pub event_type: String,
	pub task_id: Option<String>,
	pub planner_id: Option<String>,
	pub worker_id: Option<String>,
	pub data: serde_json::Value,
	pub timestamp: DateTime<Utc>,
}

/// Request to create an event subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateSubscriptionRequest {
	/// Event filter
	pub filter: EventFilterApi,
	/// Optional webhook URL for delivery
	pub webhook_url: Option<String>,
}

/// Event filter for API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EventFilterApi {
	/// Event types to include
	pub event_types: Option<Vec<String>>,
	/// Planner IDs to include
	pub planner_ids: Option<Vec<String>>,
	/// Task IDs to include
	pub task_ids: Option<Vec<String>>,
}

/// Response for subscription creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SubscriptionResponse {
	pub subscription_id: String,
	pub filter: EventFilterApi,
	pub created_at: DateTime<Utc>,
}
