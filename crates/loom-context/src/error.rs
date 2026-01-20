// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for the context management logic layer.

use thiserror::Error;

/// Errors that can occur in context management operations.
#[derive(Debug, Error)]
pub enum ContextError {
	#[error("memory item not found: {0}")]
	MemoryNotFound(String),

	#[error("context snapshot not found: {0}")]
	SnapshotNotFound(String),

	#[error("verification failed: {0}")]
	VerificationFailed(String),

	#[error("resource budget exceeded: {0}")]
	BudgetExceeded(String),

	#[error("drift detection error: {0}")]
	DriftError(String),

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

impl From<loom_context_core::ContextError> for ContextError {
	fn from(err: loom_context_core::ContextError) -> Self {
		match err {
			loom_context_core::ContextError::MemoryNotFound(s) => ContextError::MemoryNotFound(s),
			loom_context_core::ContextError::SnapshotNotFound(s) => {
				ContextError::SnapshotNotFound(s)
			}
			loom_context_core::ContextError::BudgetExceeded(s) => ContextError::BudgetExceeded(s),
			loom_context_core::ContextError::DriftDetected(s) => ContextError::DriftError(s),
			loom_context_core::ContextError::Serialization(s) => ContextError::Serialization(s),
			loom_context_core::ContextError::Internal(s) => ContextError::Internal(s),
			loom_context_core::ContextError::VerificationNotFound(s) => {
				ContextError::VerificationFailed(s)
			}
			loom_context_core::ContextError::DriftSignalNotFound(s) => ContextError::DriftError(s),
		}
	}
}

/// A specialized `Result` type for context management operations.
pub type Result<T> = std::result::Result<T, ContextError>;
