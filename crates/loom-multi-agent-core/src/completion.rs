// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Task completion types for the multi-agent system.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{TaskId, WorkerId};
use crate::task::ContextSnapshot;

/// Result of completing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskCompletion {
	/// Task that was completed
	pub task_id: TaskId,
	/// Worker that completed it
	pub worker_id: WorkerId,
	/// Completion timestamp
	pub completed_at: DateTime<Utc>,
	/// Duration in milliseconds
	pub duration_ms: u64,
	/// Output
	pub output: TaskOutput,
	/// Files changed
	pub changes: Vec<FileChange>,
	/// Commit SHA (if committed)
	pub commit_sha: Option<String>,
	/// Verification results
	pub verifications: Vec<VerificationResult>,
	/// Context at completion (for planner wake-up)
	pub context_snapshot: ContextSnapshot,
}

/// Output produced by completing a task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskOutput {
	/// Summary of what was done
	pub summary: String,
	/// Detailed notes
	pub notes: Vec<String>,
	/// Artifacts produced
	pub artifacts: Vec<Artifact>,
	/// Suggested follow-up tasks
	pub suggested_tasks: Vec<TaskSuggestion>,
	/// Learnings for future context
	pub learnings: Vec<Learning>,
}

/// Progress information for an in-progress task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskProgress {
	/// Current step description
	pub current_step: String,
	/// Progress percentage (0-100)
	pub percentage: u8,
	/// Files modified so far
	pub files_modified: Vec<PathBuf>,
	/// Verification results so far
	pub verifications: Vec<VerificationResult>,
}

/// A file change made during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FileChange {
	/// Path to the file
	pub path: PathBuf,
	/// Type of change
	pub change_type: FileChangeType,
	/// Lines added
	pub lines_added: u32,
	/// Lines removed
	pub lines_removed: u32,
}

/// Type of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FileChangeType {
	/// File was created
	Created,
	/// File was modified
	Modified,
	/// File was deleted
	Deleted,
	/// File was renamed
	Renamed,
}

impl std::fmt::Display for FileChangeType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FileChangeType::Created => write!(f, "created"),
			FileChangeType::Modified => write!(f, "modified"),
			FileChangeType::Deleted => write!(f, "deleted"),
			FileChangeType::Renamed => write!(f, "renamed"),
		}
	}
}

/// Result of a verification check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct VerificationResult {
	/// Criterion description
	pub criterion: String,
	/// Whether it passed
	pub passed: bool,
	/// Output from verification
	pub output: Option<String>,
	/// Error message if failed
	pub error: Option<String>,
}

/// An artifact produced by task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Artifact {
	/// Artifact name
	pub name: String,
	/// Artifact type
	pub artifact_type: String,
	/// Path to artifact (if file)
	pub path: Option<PathBuf>,
	/// Content (if small enough to inline)
	pub content: Option<String>,
}

/// A suggestion for a follow-up task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TaskSuggestion {
	/// Suggested task description
	pub description: String,
	/// Suggested task type
	pub task_type: String,
	/// Priority suggestion
	pub priority: i32,
	/// Rationale for the suggestion
	pub rationale: String,
}

/// A learning captured during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Learning {
	/// Learning type
	pub learning_type: LearningType,
	/// Content
	pub content: String,
	/// Relevance to domain
	pub domain_relevance: Option<String>,
}

/// Type of learning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum LearningType {
	/// Pattern discovered in codebase
	Pattern,
	/// Constraint discovered
	Constraint,
	/// Bug or issue found
	Issue,
	/// Performance insight
	Performance,
	/// Architecture insight
	Architecture,
	/// Testing insight
	Testing,
}

impl std::fmt::Display for LearningType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			LearningType::Pattern => write!(f, "pattern"),
			LearningType::Constraint => write!(f, "constraint"),
			LearningType::Issue => write!(f, "issue"),
			LearningType::Performance => write!(f, "performance"),
			LearningType::Architecture => write!(f, "architecture"),
			LearningType::Testing => write!(f, "testing"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_file_change_type_display() {
		assert_eq!(FileChangeType::Created.to_string(), "created");
		assert_eq!(FileChangeType::Modified.to_string(), "modified");
		assert_eq!(FileChangeType::Deleted.to_string(), "deleted");
		assert_eq!(FileChangeType::Renamed.to_string(), "renamed");
	}

	#[test]
	fn test_learning_type_display() {
		assert_eq!(LearningType::Pattern.to_string(), "pattern");
		assert_eq!(LearningType::Architecture.to_string(), "architecture");
	}

	#[test]
	fn test_task_progress_default() {
		let progress = TaskProgress::default();
		assert_eq!(progress.percentage, 0);
		assert!(progress.files_modified.is_empty());
	}
}
