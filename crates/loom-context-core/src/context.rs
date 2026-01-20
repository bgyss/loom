// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Context assembly types for building optimal context windows.
//!
//! The context system provides types for assembling context from various sources
//! (task, memory, files, history) with priority-based selection and compression.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::ids::{ContextSnapshotId, OrgId};
use crate::resource::ResourceUsage;

/// A section of assembled context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextSection {
	/// Task specification.
	Task {
		id: String,
		content: String,
		priority: i32,
	},
	/// Memory content.
	Memory {
		id: String,
		content: String,
		relevance: f64,
	},
	/// File content.
	FileContent {
		id: String,
		path: PathBuf,
		content: String,
	},
	/// Conversation history.
	History {
		id: String,
		content: String,
		turn_count: u32,
	},
}

impl ContextSection {
	/// Returns the section ID.
	pub fn id(&self) -> &str {
		match self {
			Self::Task { id, .. } => id,
			Self::Memory { id, .. } => id,
			Self::FileContent { id, .. } => id,
			Self::History { id, .. } => id,
		}
	}

	/// Returns the section content.
	pub fn content(&self) -> &str {
		match self {
			Self::Task { content, .. } => content,
			Self::Memory { content, .. } => content,
			Self::FileContent { content, .. } => content,
			Self::History { content, .. } => content,
		}
	}

	/// Sets the section content.
	pub fn set_content(&mut self, new_content: String) {
		match self {
			Self::Task { content, .. } => *content = new_content,
			Self::Memory { content, .. } => *content = new_content,
			Self::FileContent { content, .. } => *content = new_content,
			Self::History { content, .. } => *content = new_content,
		}
	}

	/// Returns the section type as a string.
	pub fn section_type(&self) -> &'static str {
		match self {
			Self::Task { .. } => "task",
			Self::Memory { .. } => "memory",
			Self::FileContent { .. } => "file_content",
			Self::History { .. } => "history",
		}
	}

	/// Returns the priority for context assembly (higher = more important).
	pub fn priority(&self) -> i32 {
		match self {
			Self::Task { priority, .. } => *priority,
			Self::Memory { relevance, .. } => (*relevance * 100.0) as i32,
			Self::FileContent { .. } => 50, // Medium priority
			Self::History { turn_count, .. } => 30 - (*turn_count as i32).min(30), // More recent = higher
		}
	}

	/// Creates a task section.
	pub fn task(id: impl Into<String>, content: impl Into<String>) -> Self {
		Self::Task {
			id: id.into(),
			content: content.into(),
			priority: 100, // Highest priority
		}
	}

	/// Creates a memory section.
	pub fn memory(id: impl Into<String>, content: impl Into<String>, relevance: f64) -> Self {
		Self::Memory {
			id: id.into(),
			content: content.into(),
			relevance,
		}
	}

	/// Creates a file content section.
	pub fn file_content(id: impl Into<String>, path: PathBuf, content: impl Into<String>) -> Self {
		Self::FileContent {
			id: id.into(),
			path,
			content: content.into(),
		}
	}

	/// Creates a history section.
	pub fn history(id: impl Into<String>, content: impl Into<String>, turn_count: u32) -> Self {
		Self::History {
			id: id.into(),
			content: content.into(),
			turn_count,
		}
	}
}

/// Assembled context ready for use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledContext {
	/// Context sections in priority order.
	pub sections: Vec<ContextSection>,
	/// Total tokens used.
	pub total_tokens: u32,
	/// Budget remaining after assembly.
	pub budget_remaining: u32,
}

impl AssembledContext {
	/// Creates a new empty assembled context.
	pub fn new(budget: u32) -> Self {
		Self {
			sections: Vec::new(),
			total_tokens: 0,
			budget_remaining: budget,
		}
	}

	/// Adds a section if there's budget.
	///
	/// Returns true if the section was added, false if insufficient budget.
	pub fn add_section(&mut self, section: ContextSection, tokens: u32) -> bool {
		if tokens <= self.budget_remaining {
			self.sections.push(section);
			self.total_tokens += tokens;
			self.budget_remaining -= tokens;
			true
		} else {
			false
		}
	}

	/// Returns the content of all sections joined.
	pub fn to_string(&self) -> String {
		self.sections.iter().map(|s| s.content()).collect::<Vec<_>>().join("\n\n")
	}

	/// Returns sections of a specific type.
	pub fn sections_of_type(&self, section_type: &str) -> Vec<&ContextSection> {
		self.sections.iter().filter(|s| s.section_type() == section_type).collect()
	}
}

/// Updates to apply to context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContextUpdate {
	/// Add new information.
	Add {
		section: ContextSection,
		priority: i32,
	},
	/// Remove outdated information.
	Remove { section_id: String },
	/// Update existing section.
	Update {
		section_id: String,
		new_content: String,
	},
	/// Invalidate section (mark as stale).
	Invalidate { section_id: String, reason: String },
}

/// A snapshot of context for persistence and recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
	/// Unique identifier.
	pub id: ContextSnapshotId,
	/// Associated task ID.
	pub task_id: String,
	/// When the snapshot was taken.
	pub assembled_at: DateTime<Utc>,
	/// Total tokens in the snapshot.
	pub total_tokens: u32,
	/// The context sections.
	pub sections: Vec<ContextSection>,
	/// Resource usage at time of snapshot.
	pub resource_usage: ResourceUsage,
	/// Organization ID for multi-tenancy.
	pub org_id: OrgId,
}

impl ContextSnapshot {
	/// Creates a new context snapshot.
	pub fn new(
		org_id: OrgId,
		task_id: impl Into<String>,
		context: &AssembledContext,
		resource_usage: ResourceUsage,
	) -> Self {
		Self {
			id: ContextSnapshotId::new(),
			task_id: task_id.into(),
			assembled_at: Utc::now(),
			total_tokens: context.total_tokens,
			sections: context.sections.clone(),
			resource_usage,
			org_id,
		}
	}

	/// Restores the assembled context from this snapshot.
	pub fn restore(&self, budget: u32) -> AssembledContext {
		AssembledContext {
			sections: self.sections.clone(),
			total_tokens: self.total_tokens,
			budget_remaining: budget.saturating_sub(self.total_tokens),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn context_section_creation() {
		let task = ContextSection::task("task-1", "Implement feature X");
		assert_eq!(task.id(), "task-1");
		assert_eq!(task.section_type(), "task");
		assert_eq!(task.priority(), 100);

		let memory = ContextSection::memory("mem-1", "Pattern: error handling", 0.9);
		assert_eq!(memory.section_type(), "memory");
		assert_eq!(memory.priority(), 90);
	}

	#[test]
	fn assembled_context_budget() {
		let mut context = AssembledContext::new(1000);
		assert_eq!(context.budget_remaining, 1000);

		let section = ContextSection::task("task-1", "Test task");
		assert!(context.add_section(section, 200));
		assert_eq!(context.total_tokens, 200);
		assert_eq!(context.budget_remaining, 800);

		// Try to add section that exceeds budget
		let large_section = ContextSection::memory("mem-1", "Large content", 0.5);
		assert!(!context.add_section(large_section, 900));
		assert_eq!(context.total_tokens, 200); // Unchanged
	}

	#[test]
	fn context_snapshot_roundtrip() {
		let org_id = OrgId::new();
		let mut context = AssembledContext::new(1000);
		context.add_section(ContextSection::task("task-1", "Test task"), 100);
		context.add_section(ContextSection::memory("mem-1", "Memory content", 0.8), 150);

		let usage = ResourceUsage::new();
		let snapshot = ContextSnapshot::new(org_id, "task-123", &context, usage);

		let restored = snapshot.restore(1000);
		assert_eq!(restored.sections.len(), 2);
		assert_eq!(restored.total_tokens, 250);
		assert_eq!(restored.budget_remaining, 750);
	}

	#[test]
	fn context_section_content_update() {
		let mut section = ContextSection::task("task-1", "Original content");
		assert_eq!(section.content(), "Original content");

		section.set_content("Updated content".to_string());
		assert_eq!(section.content(), "Updated content");
	}

	#[test]
	fn sections_of_type_filter() {
		let mut context = AssembledContext::new(10000);
		context.add_section(ContextSection::task("task-1", "Task 1"), 100);
		context.add_section(ContextSection::memory("mem-1", "Memory 1", 0.9), 100);
		context.add_section(ContextSection::memory("mem-2", "Memory 2", 0.8), 100);
		context.add_section(ContextSection::history("hist-1", "History", 5), 100);

		let tasks = context.sections_of_type("task");
		assert_eq!(tasks.len(), 1);

		let memories = context.sections_of_type("memory");
		assert_eq!(memories.len(), 2);

		let history = context.sections_of_type("history");
		assert_eq!(history.len(), 1);
	}
}
