// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for the multi-agent coordination system.

use thiserror::Error;

use crate::ids::{PlannerId, TaskId, WorkerId};

/// Errors that can occur in the multi-agent system.
#[derive(Debug, Error)]
pub enum MultiAgentError {
	/// Task not found
	#[error("task not found: {0}")]
	TaskNotFound(TaskId),

	/// Worker not found
	#[error("worker not found: {0}")]
	WorkerNotFound(WorkerId),

	/// Planner not found
	#[error("planner not found: {0}")]
	PlannerNotFound(PlannerId),

	/// Project run not found
	#[error("project run not found: {0}")]
	ProjectRunNotFound(String),

	/// Domain not found
	#[error("domain not found: {0}")]
	DomainNotFound(String),

	/// Invalid task status transition
	#[error("invalid task status transition from {from} to {to}")]
	InvalidStatusTransition { from: String, to: String },

	/// Task dependency cycle detected
	#[error("dependency cycle detected involving task: {0}")]
	DependencyCycle(TaskId),

	/// File ownership conflict
	#[error("file ownership conflict: {file} is owned by {owner}")]
	FileOwnershipConflict { file: String, owner: WorkerId },

	/// Worker is busy
	#[error("worker {0} is already executing a task")]
	WorkerBusy(WorkerId),

	/// Task already assigned
	#[error("task {0} is already assigned")]
	TaskAlreadyAssigned(TaskId),

	/// Invalid task type string
	#[error("invalid task type: {0}")]
	InvalidTaskType(String),

	/// Invalid task status string
	#[error("invalid task status: {0}")]
	InvalidTaskStatus(String),

	/// Invalid project phase string
	#[error("invalid project phase: {0}")]
	InvalidProjectPhase(String),

	/// Invalid verdict string
	#[error("invalid verdict: {0}")]
	InvalidVerdict(String),

	/// Invalid agent role string
	#[error("invalid agent role: {0}")]
	InvalidAgentRole(String),

	/// Serialization error
	#[error("serialization error: {0}")]
	Serialization(String),

	/// Database error
	#[error("database error: {0}")]
	Database(String),

	/// Channel error
	#[error("channel error: {0}")]
	Channel(String),

	/// Shutdown requested
	#[error("shutdown requested")]
	Shutdown,

	/// Timeout
	#[error("operation timed out: {0}")]
	Timeout(String),
}

impl From<serde_json::Error> for MultiAgentError {
	fn from(err: serde_json::Error) -> Self {
		MultiAgentError::Serialization(err.to_string())
	}
}
