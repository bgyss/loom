// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Task types for the multi-agent system.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::completion::TaskProgress;
use crate::ids::{PlannerId, TaskId, WorkerId};

/// A unit of work in the multi-agent system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Task {
	/// Unique task identifier
	pub id: TaskId,
	/// Parent task (if subtask)
	pub parent_id: Option<TaskId>,
	/// Planner that created this task
	pub created_by: PlannerId,
	/// Task priority (higher = more important)
	pub priority: i32,
	/// Task type
	pub task_type: TaskType,
	/// Human-readable description
	pub description: String,
	/// Detailed specification
	pub specification: TaskSpecification,
	/// Files this task may modify
	pub affected_files: Vec<PathBuf>,
	/// Dependencies on other tasks
	pub dependencies: Vec<TaskId>,
	/// Current status
	pub status: TaskStatus,
	/// Deadline (if any)
	pub deadline: Option<DateTime<Utc>>,
	/// Context snapshot at creation
	pub context_snapshot: ContextSnapshot,
	/// Iteration count
	pub iteration: u32,
	/// Created timestamp
	pub created_at: DateTime<Utc>,
	/// Updated timestamp
	pub updated_at: DateTime<Utc>,
}

impl Task {
	/// Create a new task with the given parameters.
	#[must_use]
	pub fn new(
		created_by: PlannerId,
		task_type: TaskType,
		description: String,
		specification: TaskSpecification,
	) -> Self {
		let now = Utc::now();
		Self {
			id: TaskId::new(),
			parent_id: None,
			created_by,
			priority: 0,
			task_type,
			description,
			specification,
			affected_files: Vec::new(),
			dependencies: Vec::new(),
			status: TaskStatus::Pending,
			deadline: None,
			context_snapshot: ContextSnapshot::default(),
			iteration: 0,
			created_at: now,
			updated_at: now,
		}
	}

	/// Check if the task is ready to be executed (no unmet dependencies).
	#[must_use]
	pub fn is_ready(&self) -> bool {
		matches!(self.status, TaskStatus::Pending) && self.dependencies.is_empty()
	}

	/// Check if the task is currently active (assigned or in progress).
	#[must_use]
	pub fn is_active(&self) -> bool {
		matches!(
			self.status,
			TaskStatus::Assigned { .. } | TaskStatus::InProgress { .. }
		)
	}

	/// Check if the task is in a terminal state.
	#[must_use]
	pub fn is_terminal(&self) -> bool {
		matches!(
			self.status,
			TaskStatus::Completed { .. } | TaskStatus::Failed { .. } | TaskStatus::Cancelled { .. }
		)
	}
}

/// Type of task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskType {
	/// Implement new feature
	Feature { acceptance_criteria: Vec<String> },
	/// Fix a bug
	BugFix {
		reproduction_steps: Vec<String>,
		expected_behavior: String,
	},
	/// Refactor existing code
	Refactor {
		goals: Vec<String>,
		constraints: Vec<String>,
	},
	/// Write or update tests
	Test { coverage_target: Option<f64> },
	/// Documentation
	Documentation { sections: Vec<String> },
	/// Code review
	Review { changes: Vec<PathBuf> },
	/// Exploration (no code changes)
	Exploration { questions: Vec<String> },
}

impl std::fmt::Display for TaskType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TaskType::Feature { .. } => write!(f, "feature"),
			TaskType::BugFix { .. } => write!(f, "bug_fix"),
			TaskType::Refactor { .. } => write!(f, "refactor"),
			TaskType::Test { .. } => write!(f, "test"),
			TaskType::Documentation { .. } => write!(f, "documentation"),
			TaskType::Review { .. } => write!(f, "review"),
			TaskType::Exploration { .. } => write!(f, "exploration"),
		}
	}
}

/// Status of a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TaskStatus {
	/// Waiting for dependencies
	Blocked { waiting_on: Vec<TaskId> },
	/// Ready to be picked up
	Pending,
	/// Assigned to a worker
	Assigned {
		worker_id: WorkerId,
		assigned_at: DateTime<Utc>,
	},
	/// Currently being executed
	InProgress {
		worker_id: WorkerId,
		started_at: DateTime<Utc>,
		progress: TaskProgress,
	},
	/// Awaiting judge evaluation
	AwaitingReview { completed_at: DateTime<Utc> },
	/// Needs iteration based on judge feedback
	NeedsIteration { feedback: String, iteration: u32 },
	/// Successfully completed
	Completed {
		completed_at: DateTime<Utc>,
		summary: String,
	},
	/// Failed permanently
	Failed {
		failed_at: DateTime<Utc>,
		error: String,
	},
	/// Cancelled
	Cancelled {
		cancelled_at: DateTime<Utc>,
		reason: String,
	},
}

impl std::fmt::Display for TaskStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TaskStatus::Blocked { .. } => write!(f, "blocked"),
			TaskStatus::Pending => write!(f, "pending"),
			TaskStatus::Assigned { .. } => write!(f, "assigned"),
			TaskStatus::InProgress { .. } => write!(f, "in_progress"),
			TaskStatus::AwaitingReview { .. } => write!(f, "awaiting_review"),
			TaskStatus::NeedsIteration { .. } => write!(f, "needs_iteration"),
			TaskStatus::Completed { .. } => write!(f, "completed"),
			TaskStatus::Failed { .. } => write!(f, "failed"),
			TaskStatus::Cancelled { .. } => write!(f, "cancelled"),
		}
	}
}

/// Detailed task specification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskSpecification {
	/// Detailed requirements
	pub requirements: Vec<String>,
	/// Success criteria (verifiable)
	pub success_criteria: Vec<VerifiableCriterion>,
	/// Constraints
	pub constraints: Vec<String>,
	/// Hints from planner
	pub hints: Vec<String>,
	/// Relevant context
	pub context: Vec<ContextItem>,
}

/// A criterion that can be verified programmatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct VerifiableCriterion {
	/// Description
	pub description: String,
	/// Verification method
	pub verification: VerificationMethod,
}

/// Method for verifying a criterion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum VerificationMethod {
	/// Run tests
	TestPass { test_pattern: String },
	/// Check compilation
	CompilationSuccess,
	/// Lint check
	LintPass { linter: String },
	/// Type check
	TypeCheck,
	/// Custom command
	Command { cmd: String, expected_exit_code: i32 },
	/// File exists
	FileExists { path: PathBuf },
	/// Pattern match in file
	PatternMatch { path: PathBuf, pattern: String },
}

/// A piece of context relevant to a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ContextItem {
	/// Type of context
	pub context_type: String,
	/// Content
	pub content: String,
	/// Source file (if applicable)
	pub source_file: Option<PathBuf>,
}

/// Snapshot of context at a point in time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ContextSnapshot {
	/// Context items
	pub items: Vec<ContextItem>,
	/// Git commit SHA at snapshot time
	pub git_sha: Option<String>,
	/// Timestamp
	pub captured_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_task_new() {
		let planner_id = PlannerId::new();
		let task = Task::new(
			planner_id.clone(),
			TaskType::Feature {
				acceptance_criteria: vec!["Test passes".to_string()],
			},
			"Implement login".to_string(),
			TaskSpecification::default(),
		);

		assert_eq!(task.created_by, planner_id);
		assert_eq!(task.iteration, 0);
		assert!(matches!(task.status, TaskStatus::Pending));
	}

	#[test]
	fn test_task_is_ready() {
		let task = Task::new(
			PlannerId::new(),
			TaskType::Feature {
				acceptance_criteria: vec![],
			},
			"Test".to_string(),
			TaskSpecification::default(),
		);
		assert!(task.is_ready());
	}

	#[test]
	fn test_task_type_display() {
		assert_eq!(
			TaskType::Feature {
				acceptance_criteria: vec![]
			}
			.to_string(),
			"feature"
		);
		assert_eq!(
			TaskType::BugFix {
				reproduction_steps: vec![],
				expected_behavior: String::new()
			}
			.to_string(),
			"bug_fix"
		);
	}

	#[test]
	fn test_task_status_display() {
		assert_eq!(TaskStatus::Pending.to_string(), "pending");
		assert_eq!(
			TaskStatus::Blocked {
				waiting_on: vec![]
			}
			.to_string(),
			"blocked"
		);
	}
}
