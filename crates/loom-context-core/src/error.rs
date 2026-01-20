// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for the context management system.

use thiserror::Error;

/// Errors that can occur in the context management system.
#[derive(Debug, Error)]
pub enum ContextError {
	#[error("memory item not found: {0}")]
	MemoryNotFound(String),

	#[error("context snapshot not found: {0}")]
	SnapshotNotFound(String),

	#[error("verification not found: {0}")]
	VerificationNotFound(String),

	#[error("drift signal not found: {0}")]
	DriftSignalNotFound(String),

	#[error("resource budget exceeded: {0}")]
	BudgetExceeded(String),

	#[error("drift detected: {0}")]
	DriftDetected(String),

	#[error("serialization error: {0}")]
	Serialization(String),

	#[error("internal error: {0}")]
	Internal(String),
}

impl From<serde_json::Error> for ContextError {
	fn from(err: serde_json::Error) -> Self {
		ContextError::Serialization(err.to_string())
	}
}

/// A specialized `Result` type for context management operations.
pub type Result<T> = std::result::Result<T, ContextError>;
