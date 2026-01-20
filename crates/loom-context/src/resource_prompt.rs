// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Resource-aware prompt building.

use loom_context_core::{ResourceBudget, ResourceUsage};

/// Builds resource awareness sections for prompts.
pub struct ResourceAwarePromptBuilder {
	budget: ResourceBudget,
	usage: ResourceUsage,
}

impl ResourceAwarePromptBuilder {
	/// Creates a new prompt builder.
	pub fn new(budget: ResourceBudget, usage: ResourceUsage) -> Self {
		Self { budget, usage }
	}

	/// Builds the resource status section for prompts.
	pub fn build_resource_section(&self) -> String {
		let remaining = self.usage.remaining(&self.budget);
		let warnings = self.usage.is_near_limit(&self.budget, 0.8);

		let mut section = String::new();
		section.push_str("## Resource Status\n\n");

		section.push_str(&format!(
			"- Context budget: {}/{} tokens ({:.0}% used)\n",
			self.usage.context_tokens,
			self.budget.max_context_tokens,
			(self.usage.context_tokens as f64 / self.budget.max_context_tokens as f64) * 100.0
		));

		section.push_str(&format!(
			"- Time remaining: {:.1}s of {:.1}s\n",
			remaining.task_time.as_secs_f64(),
			self.budget.max_task_duration.as_secs_f64()
		));

		section.push_str(&format!(
			"- Reasoning steps: {}/{}\n",
			self.usage.reasoning_steps, self.budget.max_reasoning_steps
		));

		section.push_str(&format!(
			"- Retries available: {}/{}\n",
			remaining.retries, self.budget.max_retries
		));

		if warnings.any() {
			section.push_str("\n**Resource Warnings:**\n");
			if warnings.context_pressure {
				section.push_str(
					"- Context window near capacity. Consider summarizing or completing soon.\n",
				);
			}
			if warnings.time_pressure {
				section.push_str("- Task time limit approaching. Prioritize completion.\n");
			}
			if warnings.step_pressure {
				section.push_str("- Reasoning step limit approaching. Converge on solution.\n");
			}
			if warnings.memory_pressure {
				section.push_str("- Memory limit approaching. Consider consolidating.\n");
			}
		}

		section
	}

	/// Builds a compact resource summary for inline use.
	pub fn build_compact_summary(&self) -> String {
		let context_pct =
			(self.usage.context_tokens as f64 / self.budget.max_context_tokens as f64) * 100.0;
		let step_pct =
			(self.usage.reasoning_steps as f64 / self.budget.max_reasoning_steps as f64) * 100.0;

		format!(
			"[Resources: {:.0}% context, {:.0}% steps, {} retries left]",
			context_pct,
			step_pct,
			self.budget.max_retries.saturating_sub(self.usage.retries)
		)
	}

	/// Returns true if resources are critically low.
	pub fn is_critical(&self) -> bool {
		self.usage.is_near_limit(&self.budget, 0.95).any()
	}

	/// Returns suggested action based on resource state.
	pub fn suggested_action(&self) -> Option<&'static str> {
		let warnings = self.usage.is_near_limit(&self.budget, 0.8);

		if warnings.context_pressure && warnings.time_pressure {
			Some("CRITICAL: Wrap up immediately and report partial progress")
		} else if warnings.context_pressure {
			Some("Summarize context and remove low-relevance information")
		} else if warnings.time_pressure {
			Some("Prioritize completing the most important subtask")
		} else if warnings.step_pressure {
			Some("Combine remaining steps and move toward completion")
		} else if warnings.memory_pressure {
			Some("Consolidate memories and prune low-relevance items")
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn resource_section_basic() {
		let budget = ResourceBudget::default();
		let usage = ResourceUsage::new();

		let builder = ResourceAwarePromptBuilder::new(budget, usage);
		let section = builder.build_resource_section();

		assert!(section.contains("Resource Status"));
		assert!(section.contains("Context budget"));
		assert!(section.contains("Reasoning steps"));
	}

	#[test]
	fn resource_section_with_warnings() {
		let budget = ResourceBudget::default();
		let mut usage = ResourceUsage::new();
		usage.context_tokens = 110_000; // > 80% of 128K

		let builder = ResourceAwarePromptBuilder::new(budget, usage);
		let section = builder.build_resource_section();

		assert!(section.contains("Resource Warnings"));
		assert!(section.contains("Context window near capacity"));
	}

	#[test]
	fn compact_summary() {
		let budget = ResourceBudget::default();
		let mut usage = ResourceUsage::new();
		usage.context_tokens = 64_000; // 50%
		usage.reasoning_steps = 12; // ~48%

		let builder = ResourceAwarePromptBuilder::new(budget, usage);
		let summary = builder.build_compact_summary();

		assert!(summary.contains("50%"));
		assert!(summary.contains("3 retries"));
	}

	#[test]
	fn suggested_actions() {
		let budget = ResourceBudget::default();
		let mut usage = ResourceUsage::new();

		// No pressure
		let builder = ResourceAwarePromptBuilder::new(budget.clone(), usage.clone());
		assert!(builder.suggested_action().is_none());

		// Context pressure
		usage.context_tokens = 110_000;
		let builder = ResourceAwarePromptBuilder::new(budget.clone(), usage.clone());
		assert!(builder.suggested_action().is_some());

		// Critical (both context and time)
		usage.task_elapsed = Duration::from_secs(1600);
		let builder = ResourceAwarePromptBuilder::new(budget, usage);
		assert!(builder.suggested_action().unwrap().contains("CRITICAL"));
	}

	#[test]
	fn is_critical_threshold() {
		let budget = ResourceBudget::default();
		let mut usage = ResourceUsage::new();

		let builder = ResourceAwarePromptBuilder::new(budget.clone(), usage.clone());
		assert!(!builder.is_critical());

		usage.context_tokens = 125_000; // > 95%
		let builder = ResourceAwarePromptBuilder::new(budget, usage);
		assert!(builder.is_critical());
	}
}
