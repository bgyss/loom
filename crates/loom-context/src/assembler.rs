// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Context assembly for building optimal context windows.

use std::sync::Arc;

use loom_context_core::{
	AssembledContext, ContextSection, MemoryQuery, ResourceBudget, ResourceUsage,
};

use crate::error::Result;
use crate::store::MemoryStore;

/// Simple token estimation (4 chars per token approximation).
fn estimate_tokens(text: &str) -> u32 {
	(text.len() / 4).max(1) as u32
}

/// Assembles optimal context from various sources.
pub struct ContextAssembler {
	/// Memory store reference.
	memory: Arc<MemoryStore>,
	/// Resource budget.
	budget: ResourceBudget,
}

impl ContextAssembler {
	/// Creates a new context assembler.
	pub fn new(memory: Arc<MemoryStore>, budget: ResourceBudget) -> Self {
		Self { memory, budget }
	}

	/// Assembles optimal context for a task.
	///
	/// Priority order:
	/// 1. Task specification (always included)
	/// 2. Relevant memories (high relevance first)
	/// 3. File contents (if space permits)
	/// 4. Recent history (if space permits)
	pub async fn assemble(
		&self,
		task_id: &str,
		task_content: &str,
		usage: &ResourceUsage,
	) -> Result<AssembledContext> {
		let remaining_tokens = usage.remaining(&self.budget).context_tokens;

		// Reserve tokens for response
		let available_tokens = remaining_tokens.saturating_sub(4096);
		let mut context = AssembledContext::new(available_tokens);

		// 1. Critical: Task specification (always include)
		let task_section = ContextSection::task(format!("task-{}", task_id), task_content);
		let task_tokens = estimate_tokens(task_content);
		context.add_section(task_section, task_tokens);

		// 2. High: Relevant memories
		let memory_budget = available_tokens.saturating_sub(context.total_tokens) / 2;
		self.add_memory_sections(&mut context, memory_budget).await?;

		Ok(context)
	}

	/// Adds memory sections to context within budget.
	async fn add_memory_sections(&self, context: &mut AssembledContext, budget: u32) -> Result<()> {
		let mut used = 0u32;

		// Get relevant memories
		let query = MemoryQuery::new()
			.with_min_relevance(0.3)
			.with_limit(20);

		let memories = self.memory.retrieve(&query).await;

		for memory in memories {
			let content = format_memory_content(&memory.content);
			let tokens = estimate_tokens(&content);

			if used + tokens > budget {
				break;
			}

			let section =
				ContextSection::memory(memory.id.to_string(), content, memory.relevance_score);
			if context.add_section(section, tokens) {
				used += tokens;
			} else {
				break;
			}
		}

		Ok(())
	}

	/// Compresses context when near limits.
	pub async fn compress(&self, context: &AssembledContext) -> Result<AssembledContext> {
		let mut compressed = AssembledContext::new(context.budget_remaining + context.total_tokens);

		for section in &context.sections {
			let compressed_content = compress_content(section.content());
			let tokens = estimate_tokens(&compressed_content);

			let new_section = match section {
				ContextSection::Task { id, priority, .. } => ContextSection::Task {
					id: id.clone(),
					content: compressed_content,
					priority: *priority,
				},
				ContextSection::Memory { id, relevance, .. } => ContextSection::Memory {
					id: id.clone(),
					content: compressed_content,
					relevance: *relevance,
				},
				ContextSection::FileContent { id, path, .. } => ContextSection::FileContent {
					id: id.clone(),
					path: path.clone(),
					content: compressed_content,
				},
				ContextSection::History { id, turn_count, .. } => ContextSection::History {
					id: id.clone(),
					content: compressed_content,
					turn_count: *turn_count,
				},
			};

			compressed.add_section(new_section, tokens);
		}

		Ok(compressed)
	}

	/// Adds file content sections.
	pub fn add_file_section(
		&self,
		context: &mut AssembledContext,
		path: &std::path::Path,
		content: &str,
	) -> bool {
		let tokens = estimate_tokens(content);
		let section = ContextSection::file_content(
			format!("file-{}", path.display()),
			path.to_path_buf(),
			content,
		);
		context.add_section(section, tokens)
	}

	/// Adds history section.
	pub fn add_history_section(
		&self,
		context: &mut AssembledContext,
		history: &str,
		turn_count: u32,
	) -> bool {
		let tokens = estimate_tokens(history);
		let section = ContextSection::history(format!("history-{}", turn_count), history, turn_count);
		context.add_section(section, tokens)
	}
}

/// Formats memory content for inclusion in context.
fn format_memory_content(content: &loom_context_core::MemoryContent) -> String {
	use loom_context_core::MemoryContent;

	match content {
		MemoryContent::CodePattern {
			pattern,
			example,
			applicability,
		} => {
			format!(
				"**Pattern**: {}\n**Example**: {}\n**Applicability**: {}",
				pattern, example, applicability
			)
		}
		MemoryContent::Decision {
			context,
			decision,
			rationale,
			alternatives_considered,
		} => {
			let alts = if alternatives_considered.is_empty() {
				String::new()
			} else {
				format!(
					"\n**Alternatives**: {}",
					alternatives_considered.join(", ")
				)
			};
			format!(
				"**Context**: {}\n**Decision**: {}\n**Rationale**: {}{}",
				context, decision, rationale, alts
			)
		}
		MemoryContent::ErrorResolution {
			error_pattern,
			root_cause,
			solution,
		} => {
			format!(
				"**Error**: {}\n**Root Cause**: {}\n**Solution**: {}",
				error_pattern, root_cause, solution
			)
		}
		MemoryContent::CodebaseKnowledge {
			path,
			summary,
			key_concepts,
		} => {
			let concepts = if key_concepts.is_empty() {
				String::new()
			} else {
				format!("\n**Key Concepts**: {}", key_concepts.join(", "))
			};
			format!(
				"**Path**: {}\n**Summary**: {}{}",
				path.display(),
				summary,
				concepts
			)
		}
		MemoryContent::Constraint {
			description,
			source,
			implications,
		} => {
			let impls = if implications.is_empty() {
				String::new()
			} else {
				format!("\n**Implications**: {}", implications.join(", "))
			};
			format!(
				"**Constraint**: {}\n**Source**: {}{}",
				description, source, impls
			)
		}
		MemoryContent::InteractionLearning {
			situation,
			approach,
			outcome,
			lesson,
		} => {
			format!(
				"**Situation**: {}\n**Approach**: {}\n**Outcome**: {}\n**Lesson**: {}",
				situation, approach, outcome, lesson
			)
		}
	}
}

/// Simple compression by truncating long content.
fn compress_content(content: &str) -> String {
	const MAX_LEN: usize = 2000;

	if content.len() <= MAX_LEN {
		content.to_string()
	} else {
		let mut truncated = content[..MAX_LEN].to_string();
		truncated.push_str("...[truncated]");
		truncated
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_context_core::{MemoryContent, MemoryItem, MemorySource, MemoryType, OrgId};

	#[tokio::test]
	async fn assembler_basic() {
		let org_id = OrgId::new();
		let store = Arc::new(MemoryStore::with_defaults(org_id));

		// Add a memory item
		let content = MemoryContent::CodePattern {
			pattern: "error handling".to_string(),
			example: "Result<T, E>".to_string(),
			applicability: "fallible operations".to_string(),
		};
		let item = MemoryItem::new(org_id, MemoryType::LongTerm, content, MemorySource::Manual)
			.with_relevance(0.9);
		store.add(item).await.unwrap();

		let assembler = ContextAssembler::new(store, ResourceBudget::default());
		let usage = ResourceUsage::new();

		let context = assembler
			.assemble("test-task", "Implement error handling", &usage)
			.await
			.unwrap();

		assert!(!context.sections.is_empty());
		// Should have task section
		assert!(context.sections.iter().any(|s| s.section_type() == "task"));
	}

	#[test]
	fn estimate_tokens_basic() {
		assert_eq!(estimate_tokens("test"), 1);
		assert_eq!(estimate_tokens("hello world"), 2);
		assert_eq!(estimate_tokens("a".repeat(100).as_str()), 25);
	}

	#[test]
	fn compress_content_short() {
		let short = "short content";
		assert_eq!(compress_content(short), short);
	}

	#[test]
	fn compress_content_long() {
		let long = "a".repeat(3000);
		let compressed = compress_content(&long);
		assert!(compressed.len() < long.len());
		assert!(compressed.ends_with("...[truncated]"));
	}

	use crate::store::MemoryStore;
}
