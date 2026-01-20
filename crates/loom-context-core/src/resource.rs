// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Resource management types for bounded context operations.
//!
//! Provides explicit resource limits and tracking for agents to reason about
//! their constraints and adapt strategies accordingly.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Explicit resource limits for context operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
	/// Maximum tokens per context window.
	pub max_context_tokens: u32,
	/// Maximum tokens per single operation.
	pub max_operation_tokens: u32,
	/// Maximum time per task.
	pub max_task_duration: Duration,
	/// Maximum reasoning steps per task.
	pub max_reasoning_steps: u32,
	/// Maximum retries before escalation.
	pub max_retries: u32,
	/// Memory budget for persistent state.
	pub max_memory_items: usize,
}

impl Default for ResourceBudget {
	fn default() -> Self {
		Self {
			max_context_tokens: 128_000,
			max_operation_tokens: 16_000,
			max_task_duration: Duration::from_secs(1800), // 30 minutes
			max_reasoning_steps: 25,
			max_retries: 3,
			max_memory_items: 1000,
		}
	}
}

impl ResourceBudget {
	/// Creates a new resource budget with default values.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the maximum context tokens.
	pub fn with_max_context_tokens(mut self, tokens: u32) -> Self {
		self.max_context_tokens = tokens;
		self
	}

	/// Sets the maximum operation tokens.
	pub fn with_max_operation_tokens(mut self, tokens: u32) -> Self {
		self.max_operation_tokens = tokens;
		self
	}

	/// Sets the maximum task duration.
	pub fn with_max_task_duration(mut self, duration: Duration) -> Self {
		self.max_task_duration = duration;
		self
	}

	/// Sets the maximum reasoning steps.
	pub fn with_max_reasoning_steps(mut self, steps: u32) -> Self {
		self.max_reasoning_steps = steps;
		self
	}

	/// Sets the maximum retries.
	pub fn with_max_retries(mut self, retries: u32) -> Self {
		self.max_retries = retries;
		self
	}

	/// Sets the maximum memory items.
	pub fn with_max_memory_items(mut self, items: usize) -> Self {
		self.max_memory_items = items;
		self
	}
}

/// Current resource usage tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
	/// Current context token usage.
	pub context_tokens: u32,
	/// Tokens used in current operation.
	pub operation_tokens: u32,
	/// Time elapsed on current task.
	pub task_elapsed: Duration,
	/// Reasoning steps taken.
	pub reasoning_steps: u32,
	/// Current retry count.
	pub retries: u32,
	/// Current memory item count.
	pub memory_items: usize,
}

impl ResourceUsage {
	/// Creates a new empty resource usage tracker.
	pub fn new() -> Self {
		Self::default()
	}

	/// Check if near budget limits.
	///
	/// # Arguments
	/// * `budget` - The resource budget to compare against
	/// * `threshold` - The percentage threshold (0.0 - 1.0) at which to trigger warnings
	pub fn is_near_limit(&self, budget: &ResourceBudget, threshold: f32) -> ResourceWarnings {
		let mut warnings = ResourceWarnings::default();

		if self.context_tokens as f32 > budget.max_context_tokens as f32 * threshold {
			warnings.context_pressure = true;
		}
		if self.task_elapsed > budget.max_task_duration.mul_f32(threshold) {
			warnings.time_pressure = true;
		}
		if self.reasoning_steps as f32 > budget.max_reasoning_steps as f32 * threshold {
			warnings.step_pressure = true;
		}
		if self.memory_items as f32 > budget.max_memory_items as f32 * threshold {
			warnings.memory_pressure = true;
		}

		warnings
	}

	/// Calculate remaining budget.
	pub fn remaining(&self, budget: &ResourceBudget) -> ResourceRemaining {
		ResourceRemaining {
			context_tokens: budget.max_context_tokens.saturating_sub(self.context_tokens),
			operation_tokens: budget.max_operation_tokens.saturating_sub(self.operation_tokens),
			task_time: budget.max_task_duration.saturating_sub(self.task_elapsed),
			reasoning_steps: budget.max_reasoning_steps.saturating_sub(self.reasoning_steps),
			retries: budget.max_retries.saturating_sub(self.retries),
			memory_items: budget.max_memory_items.saturating_sub(self.memory_items),
		}
	}

	/// Records a reasoning step.
	pub fn record_step(&mut self) {
		self.reasoning_steps += 1;
	}

	/// Records a retry attempt.
	pub fn record_retry(&mut self) {
		self.retries += 1;
	}

	/// Updates the context token count.
	pub fn set_context_tokens(&mut self, tokens: u32) {
		self.context_tokens = tokens;
	}

	/// Updates the operation token count.
	pub fn set_operation_tokens(&mut self, tokens: u32) {
		self.operation_tokens = tokens;
	}

	/// Updates the task elapsed time.
	pub fn set_task_elapsed(&mut self, elapsed: Duration) {
		self.task_elapsed = elapsed;
	}

	/// Updates the memory item count.
	pub fn set_memory_items(&mut self, count: usize) {
		self.memory_items = count;
	}
}

/// Resource pressure warnings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceWarnings {
	/// Context window near capacity.
	pub context_pressure: bool,
	/// Task time limit approaching.
	pub time_pressure: bool,
	/// Reasoning step limit approaching.
	pub step_pressure: bool,
	/// Memory item limit approaching.
	pub memory_pressure: bool,
}

impl ResourceWarnings {
	/// Returns true if any warning is active.
	pub fn any(&self) -> bool {
		self.context_pressure || self.time_pressure || self.step_pressure || self.memory_pressure
	}

	/// Returns true if no warnings are active.
	pub fn none(&self) -> bool {
		!self.any()
	}

	/// Returns a list of active warning descriptions.
	pub fn active_warnings(&self) -> Vec<&'static str> {
		let mut warnings = Vec::new();
		if self.context_pressure {
			warnings.push("context_pressure");
		}
		if self.time_pressure {
			warnings.push("time_pressure");
		}
		if self.step_pressure {
			warnings.push("step_pressure");
		}
		if self.memory_pressure {
			warnings.push("memory_pressure");
		}
		warnings
	}
}

/// Remaining resource budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRemaining {
	/// Remaining context tokens.
	pub context_tokens: u32,
	/// Remaining operation tokens.
	pub operation_tokens: u32,
	/// Remaining task time.
	pub task_time: Duration,
	/// Remaining reasoning steps.
	pub reasoning_steps: u32,
	/// Remaining retries.
	pub retries: u32,
	/// Remaining memory items.
	pub memory_items: usize,
}

impl ResourceRemaining {
	/// Returns true if all resources are exhausted.
	pub fn all_exhausted(&self) -> bool {
		self.context_tokens == 0
			|| self.task_time.is_zero()
			|| self.reasoning_steps == 0
			|| self.retries == 0
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	#[test]
	fn default_budget_values() {
		let budget = ResourceBudget::default();
		assert_eq!(budget.max_context_tokens, 128_000);
		assert_eq!(budget.max_operation_tokens, 16_000);
		assert_eq!(budget.max_task_duration, Duration::from_secs(1800));
		assert_eq!(budget.max_reasoning_steps, 25);
		assert_eq!(budget.max_retries, 3);
		assert_eq!(budget.max_memory_items, 1000);
	}

	#[test]
	fn resource_warnings_near_limit() {
		let budget = ResourceBudget::default();
		let mut usage = ResourceUsage::new();

		// No warnings at low usage
		let warnings = usage.is_near_limit(&budget, 0.8);
		assert!(!warnings.any());

		// Context pressure at 85% usage
		usage.context_tokens = 110_000;
		let warnings = usage.is_near_limit(&budget, 0.8);
		assert!(warnings.context_pressure);
		assert!(!warnings.time_pressure);

		// Step pressure at 85% usage
		usage.reasoning_steps = 22;
		let warnings = usage.is_near_limit(&budget, 0.8);
		assert!(warnings.step_pressure);
	}

	#[test]
	fn resource_remaining_calculation() {
		let budget = ResourceBudget::default();
		let mut usage = ResourceUsage::new();
		usage.context_tokens = 50_000;
		usage.reasoning_steps = 10;
		usage.retries = 1;

		let remaining = usage.remaining(&budget);
		assert_eq!(remaining.context_tokens, 78_000);
		assert_eq!(remaining.reasoning_steps, 15);
		assert_eq!(remaining.retries, 2);
	}

	#[test]
	fn resource_usage_tracking() {
		let mut usage = ResourceUsage::new();
		assert_eq!(usage.reasoning_steps, 0);

		usage.record_step();
		usage.record_step();
		assert_eq!(usage.reasoning_steps, 2);

		usage.record_retry();
		assert_eq!(usage.retries, 1);
	}

	proptest! {
		#[test]
		fn remaining_never_underflows(
			context_tokens in 0u32..200_000,
			reasoning_steps in 0u32..100,
			retries in 0u32..10,
		) {
			let budget = ResourceBudget::default();
			let mut usage = ResourceUsage::new();
			usage.context_tokens = context_tokens;
			usage.reasoning_steps = reasoning_steps;
			usage.retries = retries;

			let remaining = usage.remaining(&budget);
			// saturating_sub ensures no underflow
			prop_assert!(remaining.context_tokens <= budget.max_context_tokens);
			prop_assert!(remaining.reasoning_steps <= budget.max_reasoning_steps);
			prop_assert!(remaining.retries <= budget.max_retries);
		}

		#[test]
		fn warnings_threshold_consistency(
			threshold in 0.0f32..=1.0,
			usage_pct in 0.0f32..=1.5,
		) {
			let budget = ResourceBudget::default();
			let mut usage = ResourceUsage::new();
			usage.context_tokens = (budget.max_context_tokens as f32 * usage_pct) as u32;

			let warnings = usage.is_near_limit(&budget, threshold);
			if usage_pct > threshold {
				prop_assert!(warnings.context_pressure);
			} else {
				prop_assert!(!warnings.context_pressure);
			}
		}
	}
}
