// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for the context server repository layer.

use thiserror::Error;

/// Errors that can occur in context database operations.
#[derive(Debug, Error)]
pub enum ContextDbError {
	#[error("memory item not found: {0}")]
	MemoryNotFound(String),

	#[error("verification not found: {0}")]
	VerificationNotFound(String),

	#[error("drift signal not found: {0}")]
	DriftSignalNotFound(String),

	#[error("context snapshot not found: {0}")]
	SnapshotNotFound(String),

	#[error("conflict: {0}")]
	Conflict(String),

	#[error("database error: {0}")]
	Sqlx(#[from] sqlx::Error),

	#[error("serialization error: {0}")]
	Serialization(String),
}

impl From<serde_json::Error> for ContextDbError {
	fn from(err: serde_json::Error) -> Self {
		ContextDbError::Serialization(err.to_string())
	}
}

/// A specialized `Result` type for context database operations.
pub type Result<T> = std::result::Result<T, ContextDbError>;
