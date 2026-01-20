// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Server-side implementation for Loom multi-agent coordination.
//!
//! This crate provides the database repository, task queue, and file ownership
//! management for the multi-agent coordination system.
//!
//! ## Key Components
//!
//! - [`MultiAgentRepository`] - CRUD operations for all multi-agent entities
//! - [`TaskQueue`] - Priority queue with dependency resolution
//! - [`FileOwnershipManager`] - Conflict prevention through file ownership

pub mod error;
pub mod file_ownership;
pub mod repository;
pub mod task_queue;

pub use error::{MultiAgentServerError, Result};
pub use file_ownership::FileOwnershipManager;
pub use repository::{MultiAgentRepository, SqliteMultiAgentRepository};
pub use task_queue::TaskQueue;
