// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Core types for the Loom context management system.
//!
//! This crate provides shared types for maintaining agent context integrity over
//! extended operations, including memory management, resource tracking, drift detection,
//! verification, and strategy selection.
//!
//! # Overview
//!
//! The context management system supports:
//! - Structured memory with different retention patterns (working, short-term, long-term, episodic)
//! - Bounded resource awareness to prevent context bloat
//! - Drift detection and correction to maintain reasoning quality
//! - Verifiable feedback loops grounding reasoning in executable reality
//! - Dynamic strategy selection based on task type and performance history
//!
//! # Example
//!
//! ```
//! use loom_context_core::{
//!     MemoryItem, MemoryType, MemoryContent, MemorySource,
//!     ResourceBudget, ResourceUsage, OrgId,
//! };
//!
//! // Create a memory item
//! let org_id = OrgId::new();
//! let content = MemoryContent::CodePattern {
//!     pattern: "error handling".to_string(),
//!     example: "Result<T, E>".to_string(),
//!     applicability: "functions that can fail".to_string(),
//! };
//! let item = MemoryItem::new(org_id, MemoryType::LongTerm, content, MemorySource::Manual);
//!
//! // Track resource usage
//! let budget = ResourceBudget::default();
//! let mut usage = ResourceUsage::new();
//! usage.set_context_tokens(50_000);
//!
//! let warnings = usage.is_near_limit(&budget, 0.8);
//! if warnings.any() {
//!     println!("Resource pressure detected!");
//! }
//! ```

pub mod context;
pub mod drift;
pub mod error;
pub mod ids;
pub mod memory;
pub mod resource;
pub mod strategy;
pub mod verification;

// Re-export commonly used types
pub use context::{AssembledContext, ContextSection, ContextSnapshot, ContextUpdate};
pub use drift::{CorrectionAction, DriftDetectorConfig, DriftSignal, DriftSignalRecord};
pub use error::{ContextError, Result};
pub use ids::{ContextSnapshotId, DriftSignalId, MemoryId, OrgId, VerificationId};
pub use memory::{MemoryContent, MemoryItem, MemoryQuery, MemorySource, MemoryType};
pub use resource::{ResourceBudget, ResourceRemaining, ResourceUsage, ResourceWarnings};
pub use strategy::{PerformanceStats, ReasoningStrategy, StrategyFeedback, TaskType};
pub use verification::{Claim, ClaimType, FeedbackSummary, VerificationResult};

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	proptest! {
		#[test]
		fn memory_item_with_tags_preserves_all(
			tag1 in "[a-z]{1,10}",
			tag2 in "[a-z]{1,10}",
		) {
			let org_id = OrgId::new();
			let content = MemoryContent::Constraint {
				description: "test".to_string(),
				source: "test".to_string(),
				implications: vec![],
			};
			let item = MemoryItem::new(org_id, MemoryType::ShortTerm, content, MemorySource::Manual)
				.with_tag(&tag1)
				.with_tag(&tag2);

			prop_assert!(item.tags.contains(&tag1));
			prop_assert!(item.tags.contains(&tag2));
		}

		#[test]
		fn resource_budget_builder_preserves_values(
			tokens in 1000u32..500_000,
			steps in 1u32..100,
			retries in 1u32..10,
		) {
			let budget = ResourceBudget::new()
				.with_max_context_tokens(tokens)
				.with_max_reasoning_steps(steps)
				.with_max_retries(retries);

			prop_assert_eq!(budget.max_context_tokens, tokens);
			prop_assert_eq!(budget.max_reasoning_steps, steps);
			prop_assert_eq!(budget.max_retries, retries);
		}
	}
}
