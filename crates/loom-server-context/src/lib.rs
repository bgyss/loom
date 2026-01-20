// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Server-side repository layer for Loom context management.
//!
//! This crate provides SQLite persistence for context management data,
//! including memories, verifications, drift signals, and strategy performance.
//!
//! # Overview
//!
//! The repository layer persists:
//! - Memory items with decay and consolidation
//! - Verification results for feedback integration
//! - Drift signal records for debugging and correction
//! - Strategy performance statistics for optimization
//! - Context snapshots for recovery
//!
//! # Example
//!
//! ```ignore
//! use loom_server_context::{SqliteContextRepository, ContextRepository};
//! use loom_context_core::{MemoryItem, MemoryType, MemoryContent, MemorySource, OrgId};
//! use sqlx::SqlitePool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = SqlitePool::connect("sqlite::memory:").await?;
//! let repo = SqliteContextRepository::new(pool);
//!
//! // Create a memory item
//! let org_id = OrgId::new();
//! let content = MemoryContent::CodePattern {
//!     pattern: "error handling".to_string(),
//!     example: "Result<T, E>".to_string(),
//!     applicability: "fallible operations".to_string(),
//! };
//! let item = MemoryItem::new(org_id, MemoryType::ShortTerm, content, MemorySource::Manual);
//!
//! let id = repo.create_memory(&item).await?;
//! let retrieved = repo.get_memory(&id, &org_id).await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod repository;

pub use error::{ContextDbError, Result};
pub use repository::{ContextRepository, SqliteContextRepository};
