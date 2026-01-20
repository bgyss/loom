// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Strategy types for dynamic reasoning approach selection.
//!
//! Based on CodeAdapt research showing that models perform better when they can
//! adapt their strategy based on task type and intermediate results.

use serde::{Deserialize, Serialize};

/// Reasoning strategies that agents can employ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningStrategy {
	/// Single-turn reasoning for simple tasks.
	ChainOfThought,
	/// Code-centric reasoning with execution.
	ProgramOfThought,
	/// Iterative decomposition and refinement.
	StepwiseRefinement,
	/// Try multiple approaches in parallel.
	ParallelExploration { branches: u32 },
	/// Verify each reasoning step.
	VerifiedReasoning,
}

impl ReasoningStrategy {
	/// Returns the strategy name as a string.
	pub fn name(&self) -> &'static str {
		match self {
			Self::ChainOfThought => "chain_of_thought",
			Self::ProgramOfThought => "program_of_thought",
			Self::StepwiseRefinement => "stepwise_refinement",
			Self::ParallelExploration { .. } => "parallel_exploration",
			Self::VerifiedReasoning => "verified_reasoning",
		}
	}

	/// Returns a description of when to use this strategy.
	pub fn description(&self) -> &'static str {
		match self {
			Self::ChainOfThought => {
				"Single-turn reasoning for simple, well-defined tasks"
			}
			Self::ProgramOfThought => {
				"Code-centric approach with execution verification for implementation tasks"
			}
			Self::StepwiseRefinement => {
				"Iterative decomposition for complex multi-step tasks"
			}
			Self::ParallelExploration { .. } => {
				"Multiple parallel approaches for uncertain or exploratory tasks"
			}
			Self::VerifiedReasoning => {
				"Step-by-step verification for high-stakes or bug-fixing tasks"
			}
		}
	}

	/// Returns the relative resource cost (1-5, 5 being most expensive).
	pub fn resource_cost(&self) -> u8 {
		match self {
			Self::ChainOfThought => 1,
			Self::ProgramOfThought => 3,
			Self::StepwiseRefinement => 3,
			Self::ParallelExploration { branches } => {
				2 + (*branches as u8).min(3)
			}
			Self::VerifiedReasoning => 4,
		}
	}

	/// Returns true if this strategy requires execution capability.
	pub fn requires_execution(&self) -> bool {
		matches!(self, Self::ProgramOfThought | Self::VerifiedReasoning)
	}
}

impl Default for ReasoningStrategy {
	fn default() -> Self {
		Self::ChainOfThought
	}
}

impl std::fmt::Display for ReasoningStrategy {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name())
	}
}

/// Feedback signals that may trigger strategy adaptation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StrategyFeedback {
	/// High verification failure rate.
	HighFailureRate { rate: f64 },
	/// No progress being made.
	NoProgress { steps_without_progress: u32 },
	/// Resource pressure (near limits).
	ResourcePressure { resource_type: String },
	/// Task complexity higher than expected.
	HighComplexity { complexity_score: f64 },
	/// Verification success after failures.
	RecoveredFromFailure,
}

impl StrategyFeedback {
	/// Returns the feedback type as a string.
	pub fn feedback_type(&self) -> &'static str {
		match self {
			Self::HighFailureRate { .. } => "high_failure_rate",
			Self::NoProgress { .. } => "no_progress",
			Self::ResourcePressure { .. } => "resource_pressure",
			Self::HighComplexity { .. } => "high_complexity",
			Self::RecoveredFromFailure => "recovered_from_failure",
		}
	}
}

/// Task types that influence strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
	/// Bug fix task.
	BugFix,
	/// Feature implementation.
	Feature,
	/// Code refactoring.
	Refactor,
	/// Test writing.
	Test,
	/// Code exploration/understanding.
	Exploration,
	/// Documentation.
	Documentation,
	/// Configuration/setup.
	Configuration,
	/// Unknown/general.
	Unknown,
}

impl TaskType {
	/// Returns the task type as a string.
	pub fn name(&self) -> &'static str {
		match self {
			Self::BugFix => "bug_fix",
			Self::Feature => "feature",
			Self::Refactor => "refactor",
			Self::Test => "test",
			Self::Exploration => "exploration",
			Self::Documentation => "documentation",
			Self::Configuration => "configuration",
			Self::Unknown => "unknown",
		}
	}

	/// Returns the default strategy for this task type.
	pub fn default_strategy(&self) -> ReasoningStrategy {
		match self {
			Self::BugFix => ReasoningStrategy::VerifiedReasoning,
			Self::Feature => ReasoningStrategy::StepwiseRefinement,
			Self::Refactor => ReasoningStrategy::ProgramOfThought,
			Self::Test => ReasoningStrategy::ProgramOfThought,
			Self::Exploration => ReasoningStrategy::ParallelExploration { branches: 3 },
			Self::Documentation => ReasoningStrategy::ChainOfThought,
			Self::Configuration => ReasoningStrategy::ChainOfThought,
			Self::Unknown => ReasoningStrategy::StepwiseRefinement,
		}
	}
}

impl Default for TaskType {
	fn default() -> Self {
		Self::Unknown
	}
}

impl std::fmt::Display for TaskType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name())
	}
}

impl std::str::FromStr for TaskType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"bug_fix" => Ok(Self::BugFix),
			"feature" => Ok(Self::Feature),
			"refactor" => Ok(Self::Refactor),
			"test" => Ok(Self::Test),
			"exploration" => Ok(Self::Exploration),
			"documentation" => Ok(Self::Documentation),
			"configuration" => Ok(Self::Configuration),
			"unknown" => Ok(Self::Unknown),
			_ => Err(format!("unknown task type: {}", s)),
		}
	}
}

/// Performance statistics for a strategy on a task type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceStats {
	/// Number of attempts with this strategy.
	pub attempts: u32,
	/// Number of successful completions.
	pub successes: u32,
	/// Average duration in milliseconds.
	pub avg_duration_ms: Option<u64>,
	/// Average tokens used.
	pub avg_tokens_used: Option<u64>,
}

impl PerformanceStats {
	/// Creates new empty performance stats.
	pub fn new() -> Self {
		Self::default()
	}

	/// Records an attempt with the given outcome.
	pub fn record(&mut self, success: bool, duration_ms: u64, tokens_used: u64) {
		self.attempts += 1;
		if success {
			self.successes += 1;
		}

		// Update running averages
		if let Some(avg) = self.avg_duration_ms {
			self.avg_duration_ms = Some((avg * (self.attempts - 1) as u64 + duration_ms) / self.attempts as u64);
		} else {
			self.avg_duration_ms = Some(duration_ms);
		}

		if let Some(avg) = self.avg_tokens_used {
			self.avg_tokens_used = Some((avg * (self.attempts - 1) as u64 + tokens_used) / self.attempts as u64);
		} else {
			self.avg_tokens_used = Some(tokens_used);
		}
	}

	/// Returns the success rate (0.0 - 1.0).
	pub fn success_rate(&self) -> f64 {
		if self.attempts == 0 {
			0.0
		} else {
			self.successes as f64 / self.attempts as f64
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reasoning_strategy_properties() {
		let cot = ReasoningStrategy::ChainOfThought;
		assert_eq!(cot.name(), "chain_of_thought");
		assert!(!cot.requires_execution());
		assert_eq!(cot.resource_cost(), 1);

		let pot = ReasoningStrategy::ProgramOfThought;
		assert!(pot.requires_execution());

		let parallel = ReasoningStrategy::ParallelExploration { branches: 3 };
		assert!(parallel.resource_cost() > cot.resource_cost());
	}

	#[test]
	fn task_type_default_strategies() {
		assert_eq!(
			TaskType::BugFix.default_strategy(),
			ReasoningStrategy::VerifiedReasoning
		);
		assert_eq!(
			TaskType::Feature.default_strategy(),
			ReasoningStrategy::StepwiseRefinement
		);
		assert_eq!(
			TaskType::Refactor.default_strategy(),
			ReasoningStrategy::ProgramOfThought
		);
	}

	#[test]
	fn task_type_roundtrip() {
		for tt in [
			TaskType::BugFix,
			TaskType::Feature,
			TaskType::Refactor,
			TaskType::Test,
			TaskType::Exploration,
			TaskType::Documentation,
			TaskType::Configuration,
			TaskType::Unknown,
		] {
			let s = tt.name();
			let parsed: TaskType = s.parse().unwrap();
			assert_eq!(tt, parsed);
		}
	}

	#[test]
	fn performance_stats_recording() {
		let mut stats = PerformanceStats::new();
		assert_eq!(stats.attempts, 0);
		assert!((stats.success_rate() - 0.0).abs() < f64::EPSILON);

		stats.record(true, 1000, 500);
		assert_eq!(stats.attempts, 1);
		assert_eq!(stats.successes, 1);
		assert_eq!(stats.avg_duration_ms, Some(1000));

		stats.record(false, 2000, 1000);
		assert_eq!(stats.attempts, 2);
		assert_eq!(stats.successes, 1);
		assert!((stats.success_rate() - 0.5).abs() < f64::EPSILON);
		// Average of 1000 and 2000
		assert_eq!(stats.avg_duration_ms, Some(1500));
	}

	#[test]
	fn strategy_feedback_types() {
		let feedbacks = vec![
			StrategyFeedback::HighFailureRate { rate: 0.7 },
			StrategyFeedback::NoProgress {
				steps_without_progress: 5,
			},
			StrategyFeedback::ResourcePressure {
				resource_type: "context".to_string(),
			},
			StrategyFeedback::HighComplexity {
				complexity_score: 0.9,
			},
			StrategyFeedback::RecoveredFromFailure,
		];

		for feedback in feedbacks {
			assert!(!feedback.feedback_type().is_empty());
		}
	}
}
