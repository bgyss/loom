// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Memory types for the context management system.
//!
//! The memory system provides structured storage for different types of
//! knowledge that agents accumulate during operation, including code patterns,
//! decisions, error resolutions, and codebase knowledge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::ids::MemoryId;

/// Type of memory determining retention and access patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
	/// Immediate working memory (current task).
	Working,
	/// Short-term memory (recent tasks, decays over time).
	ShortTerm,
	/// Long-term memory (important learnings, persists).
	LongTerm,
	/// Episodic memory (specific events, searchable).
	Episodic,
}

impl std::fmt::Display for MemoryType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Working => write!(f, "working"),
			Self::ShortTerm => write!(f, "short_term"),
			Self::LongTerm => write!(f, "long_term"),
			Self::Episodic => write!(f, "episodic"),
		}
	}
}

impl std::str::FromStr for MemoryType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"working" => Ok(Self::Working),
			"short_term" => Ok(Self::ShortTerm),
			"long_term" => Ok(Self::LongTerm),
			"episodic" => Ok(Self::Episodic),
			_ => Err(format!("unknown memory type: {}", s)),
		}
	}
}

/// Content stored in a memory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryContent {
	/// Code pattern or solution.
	CodePattern {
		pattern: String,
		example: String,
		applicability: String,
	},
	/// Architectural decision.
	Decision {
		context: String,
		decision: String,
		rationale: String,
		alternatives_considered: Vec<String>,
	},
	/// Error and resolution.
	ErrorResolution {
		error_pattern: String,
		root_cause: String,
		solution: String,
	},
	/// File/codebase knowledge.
	CodebaseKnowledge {
		path: PathBuf,
		summary: String,
		key_concepts: Vec<String>,
	},
	/// Constraint or requirement.
	Constraint {
		description: String,
		source: String,
		implications: Vec<String>,
	},
	/// Interaction learning.
	InteractionLearning {
		situation: String,
		approach: String,
		outcome: String,
		lesson: String,
	},
}

impl MemoryContent {
	/// Returns the content type as a string for storage.
	pub fn content_type(&self) -> &'static str {
		match self {
			Self::CodePattern { .. } => "code_pattern",
			Self::Decision { .. } => "decision",
			Self::ErrorResolution { .. } => "error_resolution",
			Self::CodebaseKnowledge { .. } => "codebase_knowledge",
			Self::Constraint { .. } => "constraint",
			Self::InteractionLearning { .. } => "interaction_learning",
		}
	}
}

/// Source of a memory item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
	/// From a task execution.
	Task { task_id: String },
	/// From verification feedback.
	Verification { verification_id: String },
	/// From a conversation.
	Conversation { conversation_id: String },
	/// Manually added.
	Manual,
}

impl MemorySource {
	/// Returns the source type as a string for storage.
	pub fn source_type(&self) -> &'static str {
		match self {
			Self::Task { .. } => "task",
			Self::Verification { .. } => "verification",
			Self::Conversation { .. } => "conversation",
			Self::Manual => "manual",
		}
	}

	/// Returns the source ID if available.
	pub fn source_id(&self) -> Option<&str> {
		match self {
			Self::Task { task_id } => Some(task_id),
			Self::Verification { verification_id } => Some(verification_id),
			Self::Conversation { conversation_id } => Some(conversation_id),
			Self::Manual => None,
		}
	}
}

/// A memory item in the context management system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
	/// Unique identifier.
	pub id: MemoryId,
	/// Memory type.
	pub memory_type: MemoryType,
	/// Content.
	pub content: MemoryContent,
	/// Creation timestamp.
	pub created_at: DateTime<Utc>,
	/// Last accessed timestamp.
	pub last_accessed: DateTime<Utc>,
	/// Access count.
	pub access_count: u32,
	/// Relevance score (0.0 - 1.0).
	pub relevance_score: f64,
	/// Tags for retrieval.
	pub tags: Vec<String>,
	/// Source of this memory.
	pub source: MemorySource,
	/// Organization ID for multi-tenancy.
	pub org_id: crate::ids::OrgId,
}

impl MemoryItem {
	/// Creates a new memory item.
	pub fn new(
		org_id: crate::ids::OrgId,
		memory_type: MemoryType,
		content: MemoryContent,
		source: MemorySource,
	) -> Self {
		let now = Utc::now();
		Self {
			id: MemoryId::new(),
			memory_type,
			content,
			created_at: now,
			last_accessed: now,
			access_count: 1,
			relevance_score: 1.0,
			tags: Vec::new(),
			source,
			org_id,
		}
	}

	/// Adds a tag to this memory item.
	pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
		self.tags.push(tag.into());
		self
	}

	/// Adds multiple tags to this memory item.
	pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.tags.extend(tags.into_iter().map(Into::into));
		self
	}

	/// Sets the relevance score.
	pub fn with_relevance(mut self, score: f64) -> Self {
		self.relevance_score = score.clamp(0.0, 1.0);
		self
	}

	/// Records an access, updating last_accessed and access_count.
	pub fn record_access(&mut self) {
		self.last_accessed = Utc::now();
		self.access_count += 1;
	}

	/// Boosts relevance score by the given amount, clamped to 1.0.
	pub fn boost_relevance(&mut self, boost: f64) {
		self.relevance_score = (self.relevance_score + boost).min(1.0);
	}

	/// Decays relevance score by the given factor (0.0 - 1.0).
	pub fn decay_relevance(&mut self, factor: f64) {
		self.relevance_score *= factor;
	}
}

/// Query parameters for retrieving memories.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
	/// Filter by memory type.
	pub memory_type: Option<MemoryType>,
	/// Filter by tags (any match).
	pub tags: Vec<String>,
	/// Maximum results to return.
	pub max_results: Option<usize>,
	/// Minimum relevance score.
	pub min_relevance: Option<f64>,
	/// Full-text search query.
	pub search_text: Option<String>,
}

impl MemoryQuery {
	/// Creates a new empty query.
	pub fn new() -> Self {
		Self::default()
	}

	/// Filters by memory type.
	pub fn with_type(mut self, memory_type: MemoryType) -> Self {
		self.memory_type = Some(memory_type);
		self
	}

	/// Filters by tags (any match).
	pub fn with_tags(mut self, tags: Vec<String>) -> Self {
		self.tags = tags;
		self
	}

	/// Limits the number of results.
	pub fn with_limit(mut self, limit: usize) -> Self {
		self.max_results = Some(limit);
		self
	}

	/// Sets minimum relevance score.
	pub fn with_min_relevance(mut self, min: f64) -> Self {
		self.min_relevance = Some(min);
		self
	}

	/// Sets full-text search query.
	pub fn with_search(mut self, text: impl Into<String>) -> Self {
		self.search_text = Some(text.into());
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ids::OrgId;
	use proptest::prelude::*;

	#[test]
	fn memory_type_roundtrip() {
		for mt in [
			MemoryType::Working,
			MemoryType::ShortTerm,
			MemoryType::LongTerm,
			MemoryType::Episodic,
		] {
			let s = mt.to_string();
			let parsed: MemoryType = s.parse().unwrap();
			assert_eq!(mt, parsed);
		}
	}

	#[test]
	fn memory_item_creation() {
		let org_id = OrgId::new();
		let content = MemoryContent::CodePattern {
			pattern: "error handling".to_string(),
			example: "Result<T, E>".to_string(),
			applicability: "all functions that can fail".to_string(),
		};
		let item = MemoryItem::new(org_id, MemoryType::LongTerm, content, MemorySource::Manual);

		assert_eq!(item.memory_type, MemoryType::LongTerm);
		assert_eq!(item.access_count, 1);
		assert!((item.relevance_score - 1.0).abs() < f64::EPSILON);
	}

	#[test]
	fn memory_item_access_tracking() {
		let org_id = OrgId::new();
		let content = MemoryContent::Decision {
			context: "test".to_string(),
			decision: "test".to_string(),
			rationale: "test".to_string(),
			alternatives_considered: vec![],
		};
		let mut item =
			MemoryItem::new(org_id, MemoryType::ShortTerm, content, MemorySource::Manual);

		let initial_count = item.access_count;
		item.record_access();
		assert_eq!(item.access_count, initial_count + 1);
	}

	#[test]
	fn memory_item_relevance_decay() {
		let org_id = OrgId::new();
		let content = MemoryContent::ErrorResolution {
			error_pattern: "error".to_string(),
			root_cause: "cause".to_string(),
			solution: "solution".to_string(),
		};
		let mut item =
			MemoryItem::new(org_id, MemoryType::ShortTerm, content, MemorySource::Manual);

		item.decay_relevance(0.5);
		assert!((item.relevance_score - 0.5).abs() < f64::EPSILON);

		item.boost_relevance(0.3);
		assert!((item.relevance_score - 0.8).abs() < f64::EPSILON);
	}

	proptest! {
		#[test]
		fn relevance_score_stays_bounded(
			initial in 0.0..=1.0f64,
			decay in 0.0..=1.0f64,
			boost in 0.0..=1.0f64,
		) {
			let org_id = OrgId::new();
			let content = MemoryContent::Constraint {
				description: "test".to_string(),
				source: "test".to_string(),
				implications: vec![],
			};
			let mut item = MemoryItem::new(org_id, MemoryType::ShortTerm, content, MemorySource::Manual)
				.with_relevance(initial);

			item.decay_relevance(decay);
			prop_assert!(item.relevance_score >= 0.0);
			prop_assert!(item.relevance_score <= 1.0);

			item.boost_relevance(boost);
			prop_assert!(item.relevance_score >= 0.0);
			prop_assert!(item.relevance_score <= 1.0);
		}
	}
}
