// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for the multi-agent server.

use loom_multi_agent_core::MultiAgentError;
use thiserror::Error;

/// Errors that can occur in the multi-agent server.
#[derive(Debug, Error)]
pub enum MultiAgentServerError {
	/// Database error
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),

	/// Project run not found
	#[error("project run not found: {0}")]
	ProjectRunNotFound(String),

	/// Task not found
	#[error("task not found: {0}")]
	TaskNotFound(String),

	/// Worker not found
	#[error("worker not found: {0}")]
	WorkerNotFound(String),

	/// Planner not found
	#[error("planner not found: {0}")]
	PlannerNotFound(String),

	/// Domain not found
	#[error("domain not found: {0}")]
	DomainNotFound(String),

	/// Invalid data
	#[error("invalid data: {0}")]
	InvalidData(String),

	/// JSON serialization error
	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),

	/// Core error
	#[error("multi-agent core error: {0}")]
	Core(#[from] MultiAgentError),

	/// File ownership conflict
	#[error("file ownership conflict: {file} is owned by worker {owner}")]
	FileOwnershipConflict { file: String, owner: String },

	/// Task queue error
	#[error("task queue error: {0}")]
	TaskQueue(String),

	/// Channel error
	#[error("channel error: {0}")]
	Channel(String),
}

/// Result type for multi-agent server operations.
pub type Result<T> = std::result::Result<T, MultiAgentServerError>;
