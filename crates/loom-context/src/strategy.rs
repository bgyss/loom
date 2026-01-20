// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Strategy selection and adaptation.

use std::collections::HashMap;

use loom_context_core::{
	PerformanceStats, ReasoningStrategy, ResourceBudget, ResourceUsage, StrategyFeedback, TaskType,
};

/// Builds strategy-specific prompt sections to guide LLM reasoning.
pub struct StrategyPromptBuilder;

impl StrategyPromptBuilder {
	/// Creates a new strategy prompt builder.
	pub fn new() -> Self {
		Self
	}

	/// Builds a strategy instruction section for the given strategy.
	pub fn build_strategy_section(&self, strategy: &ReasoningStrategy) -> String {
		let mut section = String::new();
		section.push_str("## Reasoning Strategy\n\n");
		section.push_str(&format!(
			"**Active Strategy:** {} (resource cost: {})\n\n",
			strategy.name(),
			strategy.resource_cost()
		));
		section.push_str(strategy.description());
		section.push_str("\n\n");
		section.push_str("### Guidelines\n\n");
		section.push_str(&self.build_guidelines(strategy));
		section
	}

	/// Builds strategy-specific guidelines.
	fn build_guidelines(&self, strategy: &ReasoningStrategy) -> String {
		match strategy {
			ReasoningStrategy::ChainOfThought => self.chain_of_thought_guidelines(),
			ReasoningStrategy::ProgramOfThought => self.program_of_thought_guidelines(),
			ReasoningStrategy::StepwiseRefinement => self.stepwise_refinement_guidelines(),
			ReasoningStrategy::ParallelExploration { branches } => {
				self.parallel_exploration_guidelines(*branches)
			}
			ReasoningStrategy::VerifiedReasoning => self.verified_reasoning_guidelines(),
		}
	}

	fn chain_of_thought_guidelines(&self) -> String {
		r#"1. Think through the problem step-by-step in a single pass
2. State your reasoning clearly before taking action
3. Keep explanations concise and focused
4. Aim to complete the task in one coherent sequence
5. Avoid over-analyzing; trust your initial assessment
"#
		.to_string()
	}

	fn program_of_thought_guidelines(&self) -> String {
		r#"1. Focus on writing and executing code to solve the problem
2. Use execution results to guide your next steps
3. Prefer code verification over manual reasoning
4. Test hypotheses through actual execution
5. Let the compiler/runtime catch errors rather than over-thinking
6. Document your code to explain intent
"#
		.to_string()
	}

	fn stepwise_refinement_guidelines(&self) -> String {
		r#"1. Break the task into smaller, manageable subtasks
2. Complete each subtask fully before moving to the next
3. Review and refine after each step if needed
4. Track what has been completed and what remains
5. Consolidate progress periodically
6. Adjust the plan as new information emerges
"#
		.to_string()
	}

	fn parallel_exploration_guidelines(&self, branches: u32) -> String {
		format!(
			r#"1. Consider up to {} different approaches in parallel
2. Briefly evaluate the pros and cons of each approach
3. Select the most promising approach based on:
   - Likelihood of success
   - Resource efficiency
   - Alignment with constraints
4. Document why other approaches were rejected
5. Be willing to backtrack if the chosen approach fails
6. Keep exploration focused; don't go too deep on any one path until committed
"#,
			branches
		)
	}

	fn verified_reasoning_guidelines(&self) -> String {
		r#"1. Verify each significant step before proceeding
2. Make claims that can be tested (e.g., "this compiles", "tests pass")
3. Execute verification commands after changes
4. If verification fails, stop and diagnose before continuing
5. Prefer small, verified changes over large unverified ones
6. Document what was verified and the results
7. Never assume something works without confirmation
"#
		.to_string()
	}

	/// Returns a compact strategy hint for inline use.
	pub fn build_compact_hint(&self, strategy: &ReasoningStrategy) -> String {
		match strategy {
			ReasoningStrategy::ChainOfThought => {
				"[Strategy: Single-pass reasoning. Think through, then act.]".to_string()
			}
			ReasoningStrategy::ProgramOfThought => {
				"[Strategy: Code-centric. Write code and execute to verify.]".to_string()
			}
			ReasoningStrategy::StepwiseRefinement => {
				"[Strategy: Decompose. Complete subtasks incrementally.]".to_string()
			}
			ReasoningStrategy::ParallelExploration { branches } => {
				format!(
					"[Strategy: Explore {} approaches. Compare, then commit.]",
					branches
				)
			}
			ReasoningStrategy::VerifiedReasoning => {
				"[Strategy: Verify each step. Test before proceeding.]".to_string()
			}
		}
	}

	/// Builds an adaptation notice when strategy has changed.
	pub fn build_adaptation_notice(
		&self,
		old_strategy: &ReasoningStrategy,
		new_strategy: &ReasoningStrategy,
		feedback: &StrategyFeedback,
	) -> String {
		format!(
			"## Strategy Adaptation\n\n\
			**Previous:** {} -> **New:** {}\n\n\
			**Reason:** {}\n\n\
			Please adjust your approach according to the new strategy guidelines.\n",
			old_strategy.name(),
			new_strategy.name(),
			self.feedback_reason(feedback)
		)
	}

	fn feedback_reason(&self, feedback: &StrategyFeedback) -> String {
		match feedback {
			StrategyFeedback::HighFailureRate { rate } => {
				format!(
					"High verification failure rate ({:.0}%). Switching to more careful approach.",
					rate * 100.0
				)
			}
			StrategyFeedback::NoProgress { steps_without_progress } => {
				format!(
					"No progress detected after {} steps. Trying different approach.",
					steps_without_progress
				)
			}
			StrategyFeedback::ResourcePressure { resource_type } => {
				format!(
					"Resource pressure on {}. Switching to more efficient strategy.",
					resource_type
				)
			}
			StrategyFeedback::HighComplexity { complexity_score } => {
				format!(
					"High task complexity detected ({:.0}%). Using more thorough approach.",
					complexity_score * 100.0
				)
			}
			StrategyFeedback::RecoveredFromFailure => {
				"Recovered from previous failures. Maintaining current approach.".to_string()
			}
		}
	}
}

impl Default for StrategyPromptBuilder {
	fn default() -> Self {
		Self::new()
	}
}

/// Classifies tasks to determine appropriate strategies.
pub struct TaskClassifier;

impl TaskClassifier {
	/// Creates a new task classifier.
	pub fn new() -> Self {
		Self
	}

	/// Classifies a task based on its description.
	pub fn classify(&self, description: &str) -> TaskType {
		let desc_lower = description.to_lowercase();

		if desc_lower.contains("fix")
			|| desc_lower.contains("bug")
			|| desc_lower.contains("issue")
			|| desc_lower.contains("error")
		{
			TaskType::BugFix
		} else if desc_lower.contains("add")
			|| desc_lower.contains("implement")
			|| desc_lower.contains("create")
			|| desc_lower.contains("new feature")
		{
			TaskType::Feature
		} else if desc_lower.contains("refactor")
			|| desc_lower.contains("restructure")
			|| desc_lower.contains("reorganize")
			|| desc_lower.contains("clean up")
		{
			TaskType::Refactor
		} else if desc_lower.contains("test")
			|| desc_lower.contains("spec")
			|| desc_lower.contains("coverage")
		{
			TaskType::Test
		} else if desc_lower.contains("explore")
			|| desc_lower.contains("understand")
			|| desc_lower.contains("investigate")
			|| desc_lower.contains("analyze")
		{
			TaskType::Exploration
		} else if desc_lower.contains("doc")
			|| desc_lower.contains("readme")
			|| desc_lower.contains("comment")
		{
			TaskType::Documentation
		} else if desc_lower.contains("config")
			|| desc_lower.contains("setup")
			|| desc_lower.contains("install")
		{
			TaskType::Configuration
		} else {
			TaskType::Unknown
		}
	}
}

impl Default for TaskClassifier {
	fn default() -> Self {
		Self::new()
	}
}

/// Selects and adapts reasoning strategies.
pub struct StrategySelector {
	/// Task classifier.
	classifier: TaskClassifier,
	/// Performance history by (task type, strategy).
	history: HashMap<(TaskType, ReasoningStrategy), PerformanceStats>,
	/// Default budget for resource checks.
	budget: ResourceBudget,
}

impl StrategySelector {
	/// Creates a new strategy selector.
	pub fn new() -> Self {
		Self {
			classifier: TaskClassifier::new(),
			history: HashMap::new(),
			budget: ResourceBudget::default(),
		}
	}

	/// Creates a strategy selector with a custom budget.
	pub fn with_budget(budget: ResourceBudget) -> Self {
		Self {
			classifier: TaskClassifier::new(),
			history: HashMap::new(),
			budget,
		}
	}

	/// Selects a strategy based on task and resources.
	pub fn select(&self, task_description: &str, resources: &ResourceUsage) -> ReasoningStrategy {
		let task_type = self.classifier.classify(task_description);

		// Check if resources are constrained
		let resource_pressure = resources.is_near_limit(&self.budget, 0.7);

		if resource_pressure.any() {
			// Use simpler strategy when resources are tight
			return ReasoningStrategy::ChainOfThought;
		}

		// Use historical performance if available
		if let Some(best) = self.best_strategy_for_type(&task_type) {
			return best;
		}

		// Fall back to default for task type
		task_type.default_strategy()
	}

	/// Finds the best strategy for a task type based on history.
	fn best_strategy_for_type(&self, task_type: &TaskType) -> Option<ReasoningStrategy> {
		let strategies = [
			ReasoningStrategy::ChainOfThought,
			ReasoningStrategy::ProgramOfThought,
			ReasoningStrategy::StepwiseRefinement,
			ReasoningStrategy::ParallelExploration { branches: 2 },
			ReasoningStrategy::VerifiedReasoning,
		];

		let mut best: Option<(ReasoningStrategy, f64)> = None;

		for strategy in strategies {
			if let Some(stats) = self.history.get(&(*task_type, strategy)) {
				if stats.attempts >= 3 {
					// Need enough data
					let rate = stats.success_rate();
					if best.is_none() || rate > best.unwrap().1 {
						best = Some((strategy, rate));
					}
				}
			}
		}

		best.filter(|(_, rate)| *rate > 0.5).map(|(s, _)| s)
	}

	/// Adapts strategy based on intermediate feedback.
	pub fn adapt(
		&self,
		current: &ReasoningStrategy,
		feedback: &StrategyFeedback,
	) -> Option<ReasoningStrategy> {
		match (current, feedback) {
			// If verification is failing, switch to more careful approach
			(_, StrategyFeedback::HighFailureRate { rate }) if *rate > 0.5 => {
				Some(ReasoningStrategy::VerifiedReasoning)
			}

			// If stuck, try parallel exploration
			(ReasoningStrategy::ChainOfThought, StrategyFeedback::NoProgress { .. })
			| (ReasoningStrategy::StepwiseRefinement, StrategyFeedback::NoProgress { .. }) => {
				Some(ReasoningStrategy::ParallelExploration { branches: 2 })
			}

			// If parallel exploration isn't working, try verified reasoning
			(
				ReasoningStrategy::ParallelExploration { .. },
				StrategyFeedback::NoProgress { steps_without_progress },
			) if *steps_without_progress > 3 => Some(ReasoningStrategy::VerifiedReasoning),

			// If taking too long, simplify
			(ReasoningStrategy::ParallelExploration { .. }, StrategyFeedback::ResourcePressure { .. })
			| (ReasoningStrategy::VerifiedReasoning, StrategyFeedback::ResourcePressure { .. }) => {
				Some(ReasoningStrategy::ChainOfThought)
			}

			// High complexity suggests verified reasoning
			(_, StrategyFeedback::HighComplexity { complexity_score })
				if *complexity_score > 0.8 =>
			{
				Some(ReasoningStrategy::VerifiedReasoning)
			}

			// Recovery suggests current strategy is working
			(_, StrategyFeedback::RecoveredFromFailure) => None,

			_ => None,
		}
	}

	/// Records a strategy attempt outcome.
	pub fn record_attempt(
		&mut self,
		task_type: TaskType,
		strategy: ReasoningStrategy,
		success: bool,
		duration_ms: u64,
		tokens_used: u64,
	) {
		let stats = self
			.history
			.entry((task_type, strategy))
			.or_insert_with(PerformanceStats::new);
		stats.record(success, duration_ms, tokens_used);
	}

	/// Returns performance stats for a task type and strategy.
	pub fn get_stats(
		&self,
		task_type: &TaskType,
		strategy: &ReasoningStrategy,
	) -> Option<&PerformanceStats> {
		self.history.get(&(*task_type, *strategy))
	}

	/// Returns all stats.
	pub fn all_stats(&self) -> &HashMap<(TaskType, ReasoningStrategy), PerformanceStats> {
		&self.history
	}
}

impl Default for StrategySelector {
	fn default() -> Self {
		Self::new()
	}
}

/// Monitors task progress and generates strategy feedback signals.
pub struct StrategyFeedbackMonitor {
	/// Recent verification results (passed, total).
	recent_verifications: Vec<bool>,
	/// Steps since last measurable progress.
	steps_since_progress: u32,
	/// Last successful verification count.
	last_success_count: u32,
	/// Topics seen in recent actions.
	recent_topics: Vec<String>,
	/// Actions taken.
	action_count: u32,
	/// Maximum verifications to track for failure rate.
	max_verifications: usize,
	/// Maximum topics to track for complexity.
	max_topics: usize,
}

impl StrategyFeedbackMonitor {
	/// Creates a new strategy feedback monitor.
	pub fn new() -> Self {
		Self {
			recent_verifications: Vec::new(),
			steps_since_progress: 0,
			last_success_count: 0,
			recent_topics: Vec::new(),
			action_count: 0,
			max_verifications: 20,
			max_topics: 50,
		}
	}

	/// Records a verification result.
	pub fn record_verification(&mut self, passed: bool) {
		if self.recent_verifications.len() >= self.max_verifications {
			self.recent_verifications.remove(0);
		}
		self.recent_verifications.push(passed);

		// Track progress
		let current_successes = self.recent_verifications.iter().filter(|&&p| p).count() as u32;
		if current_successes > self.last_success_count {
			self.steps_since_progress = 0;
			self.last_success_count = current_successes;
		} else if !passed {
			self.steps_since_progress += 1;
		}
	}

	/// Records an action with its associated topics.
	pub fn record_action(&mut self, topics: Vec<String>) {
		self.action_count += 1;
		self.steps_since_progress += 1;

		for topic in topics {
			if !self.recent_topics.contains(&topic) {
				if self.recent_topics.len() >= self.max_topics {
					self.recent_topics.remove(0);
				}
				self.recent_topics.push(topic);
			}
		}
	}

	/// Records successful progress (resets no-progress counter).
	pub fn record_progress(&mut self) {
		self.steps_since_progress = 0;
	}

	/// Checks for feedback signals and returns any detected.
	pub fn check(&self, resource_usage: &ResourceUsage, budget: &ResourceBudget) -> Vec<StrategyFeedback> {
		let mut signals = Vec::new();

		// Check for high failure rate
		if let Some(rate) = self.failure_rate() {
			if rate > 0.5 {
				signals.push(StrategyFeedback::HighFailureRate { rate });
			}
		}

		// Check for no progress
		if self.steps_since_progress >= 5 {
			signals.push(StrategyFeedback::NoProgress {
				steps_without_progress: self.steps_since_progress,
			});
		}

		// Check for resource pressure
		let warnings = resource_usage.is_near_limit(budget, 0.8);
		if warnings.context_pressure {
			signals.push(StrategyFeedback::ResourcePressure {
				resource_type: "context".to_string(),
			});
		} else if warnings.time_pressure {
			signals.push(StrategyFeedback::ResourcePressure {
				resource_type: "time".to_string(),
			});
		} else if warnings.step_pressure {
			signals.push(StrategyFeedback::ResourcePressure {
				resource_type: "reasoning_steps".to_string(),
			});
		}

		// Check for high complexity (many diverse topics)
		if let Some(score) = self.complexity_score() {
			if score > 0.8 {
				signals.push(StrategyFeedback::HighComplexity {
					complexity_score: score,
				});
			}
		}

		// Check for recovery
		if self.recent_verifications.len() >= 3 {
			let last_three = &self.recent_verifications[self.recent_verifications.len() - 3..];
			let earlier = &self.recent_verifications[..self.recent_verifications.len() - 3];

			// If last 3 are all passing and earlier had failures
			if last_three.iter().all(|&p| p) && earlier.iter().any(|&p| !p) {
				signals.push(StrategyFeedback::RecoveredFromFailure);
			}
		}

		signals
	}

	/// Returns the failure rate over recent verifications, if enough data.
	pub fn failure_rate(&self) -> Option<f64> {
		if self.recent_verifications.len() < 3 {
			return None;
		}

		let failed = self.recent_verifications.iter().filter(|&&p| !p).count();
		Some(failed as f64 / self.recent_verifications.len() as f64)
	}

	/// Returns a complexity score based on topic diversity and action count.
	pub fn complexity_score(&self) -> Option<f64> {
		if self.action_count < 5 {
			return None;
		}

		// More unique topics relative to actions = higher complexity
		let topic_ratio = self.recent_topics.len() as f64 / self.action_count as f64;

		// Clamp to 0-1 range (assuming ~0.5 topics per action is moderate)
		Some((topic_ratio * 2.0).min(1.0))
	}

	/// Resets the monitor state.
	pub fn reset(&mut self) {
		self.recent_verifications.clear();
		self.steps_since_progress = 0;
		self.last_success_count = 0;
		self.recent_topics.clear();
		self.action_count = 0;
	}
}

impl Default for StrategyFeedbackMonitor {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// === StrategyPromptBuilder Tests ===

	#[test]
	fn strategy_prompt_builder_sections() {
		let builder = StrategyPromptBuilder::new();

		for strategy in [
			ReasoningStrategy::ChainOfThought,
			ReasoningStrategy::ProgramOfThought,
			ReasoningStrategy::StepwiseRefinement,
			ReasoningStrategy::ParallelExploration { branches: 3 },
			ReasoningStrategy::VerifiedReasoning,
		] {
			let section = builder.build_strategy_section(&strategy);
			assert!(section.contains("Reasoning Strategy"));
			assert!(section.contains("Guidelines"));
			assert!(section.contains(strategy.name()));
		}
	}

	#[test]
	fn strategy_prompt_builder_compact_hints() {
		let builder = StrategyPromptBuilder::new();

		let hint = builder.build_compact_hint(&ReasoningStrategy::ChainOfThought);
		assert!(hint.contains("Strategy"));
		assert!(hint.contains("Single-pass"));

		let hint = builder.build_compact_hint(&ReasoningStrategy::VerifiedReasoning);
		assert!(hint.contains("Verify"));
	}

	#[test]
	fn strategy_prompt_builder_adaptation_notice() {
		let builder = StrategyPromptBuilder::new();

		let notice = builder.build_adaptation_notice(
			&ReasoningStrategy::ChainOfThought,
			&ReasoningStrategy::VerifiedReasoning,
			&StrategyFeedback::HighFailureRate { rate: 0.7 },
		);

		assert!(notice.contains("Strategy Adaptation"));
		assert!(notice.contains("chain_of_thought"));
		assert!(notice.contains("verified_reasoning"));
		assert!(notice.contains("70%"));
	}

	// === StrategyFeedbackMonitor Tests ===

	#[test]
	fn feedback_monitor_high_failure_rate() {
		let mut monitor = StrategyFeedbackMonitor::new();

		// Record mostly failures
		monitor.record_verification(false);
		monitor.record_verification(false);
		monitor.record_verification(false);
		monitor.record_verification(true);

		let rate = monitor.failure_rate().unwrap();
		assert!((rate - 0.75).abs() < 0.01);

		let feedback = monitor.check(&ResourceUsage::new(), &ResourceBudget::default());
		assert!(feedback.iter().any(|f| matches!(f, StrategyFeedback::HighFailureRate { rate } if *rate > 0.5)));
	}

	#[test]
	fn feedback_monitor_no_progress() {
		let mut monitor = StrategyFeedbackMonitor::new();

		// Record actions without progress
		for _ in 0..6 {
			monitor.record_action(vec!["topic".to_string()]);
		}

		let feedback = monitor.check(&ResourceUsage::new(), &ResourceBudget::default());
		assert!(feedback.iter().any(|f| matches!(f, StrategyFeedback::NoProgress { .. })));
	}

	#[test]
	fn feedback_monitor_progress_resets() {
		let mut monitor = StrategyFeedbackMonitor::new();

		// Record actions without progress
		for _ in 0..4 {
			monitor.record_action(vec!["topic".to_string()]);
		}

		// Record progress
		monitor.record_progress();

		// Should not detect no-progress yet
		let feedback = monitor.check(&ResourceUsage::new(), &ResourceBudget::default());
		assert!(!feedback.iter().any(|f| matches!(f, StrategyFeedback::NoProgress { .. })));
	}

	#[test]
	fn feedback_monitor_resource_pressure() {
		let monitor = StrategyFeedbackMonitor::new();

		let mut usage = ResourceUsage::new();
		usage.context_tokens = 110_000; // > 80% of default 128K

		let feedback = monitor.check(&usage, &ResourceBudget::default());
		assert!(feedback.iter().any(|f| matches!(f, StrategyFeedback::ResourcePressure { resource_type } if resource_type == "context")));
	}

	#[test]
	fn feedback_monitor_recovery() {
		let mut monitor = StrategyFeedbackMonitor::new();

		// Earlier failures
		monitor.record_verification(true);
		monitor.record_verification(false);
		monitor.record_verification(false);

		// Recent successes
		monitor.record_verification(true);
		monitor.record_verification(true);
		monitor.record_verification(true);

		let feedback = monitor.check(&ResourceUsage::new(), &ResourceBudget::default());
		assert!(feedback.iter().any(|f| matches!(f, StrategyFeedback::RecoveredFromFailure)));
	}

	#[test]
	fn feedback_monitor_complexity() {
		let mut monitor = StrategyFeedbackMonitor::new();

		// Many diverse topics (high complexity)
		for i in 0..10 {
			monitor.record_action(vec![format!("topic_{}", i)]);
		}

		let score = monitor.complexity_score().unwrap();
		assert!(score > 0.5);
	}

	#[test]
	fn feedback_monitor_reset() {
		let mut monitor = StrategyFeedbackMonitor::new();

		monitor.record_verification(false);
		monitor.record_action(vec!["topic".to_string()]);

		assert!(!monitor.recent_verifications.is_empty());

		monitor.reset();

		assert!(monitor.recent_verifications.is_empty());
		assert_eq!(monitor.action_count, 0);
	}

	// === TaskClassifier Tests ===

	#[test]
	fn task_classification() {
		let classifier = TaskClassifier::new();

		assert_eq!(classifier.classify("fix the login bug"), TaskType::BugFix);
		assert_eq!(
			classifier.classify("add user authentication"),
			TaskType::Feature
		);
		assert_eq!(
			classifier.classify("refactor the database layer"),
			TaskType::Refactor
		);
		assert_eq!(
			classifier.classify("write tests for the API"),
			TaskType::Test
		);
		assert_eq!(
			classifier.classify("explore the codebase"),
			TaskType::Exploration
		);
		assert_eq!(
			classifier.classify("update the documentation"),
			TaskType::Documentation
		);
		assert_eq!(
			classifier.classify("configure the CI pipeline"),
			TaskType::Configuration
		);
		assert_eq!(
			classifier.classify("do something"),
			TaskType::Unknown
		);
	}

	#[test]
	fn strategy_selection_defaults() {
		let selector = StrategySelector::new();
		let usage = ResourceUsage::new();

		// Bug fix should use verified reasoning
		let strategy = selector.select("fix the authentication bug", &usage);
		assert_eq!(strategy, ReasoningStrategy::VerifiedReasoning);

		// Feature should use stepwise refinement
		let strategy = selector.select("add a new endpoint", &usage);
		assert_eq!(strategy, ReasoningStrategy::StepwiseRefinement);
	}

	#[test]
	fn strategy_selection_under_pressure() {
		let selector = StrategySelector::new();
		let mut usage = ResourceUsage::new();
		usage.context_tokens = 100_000; // High usage

		// Should use simple strategy under pressure
		let strategy = selector.select("fix the complex bug", &usage);
		assert_eq!(strategy, ReasoningStrategy::ChainOfThought);
	}

	#[test]
	fn strategy_adaptation() {
		let selector = StrategySelector::new();

		// High failure rate -> verified reasoning
		let adapted = selector.adapt(
			&ReasoningStrategy::StepwiseRefinement,
			&StrategyFeedback::HighFailureRate { rate: 0.7 },
		);
		assert_eq!(adapted, Some(ReasoningStrategy::VerifiedReasoning));

		// No progress -> parallel exploration
		let adapted = selector.adapt(
			&ReasoningStrategy::ChainOfThought,
			&StrategyFeedback::NoProgress {
				steps_without_progress: 5,
			},
		);
		assert_eq!(
			adapted,
			Some(ReasoningStrategy::ParallelExploration { branches: 2 })
		);

		// Resource pressure -> simplify
		let adapted = selector.adapt(
			&ReasoningStrategy::ParallelExploration { branches: 3 },
			&StrategyFeedback::ResourcePressure {
				resource_type: "context".to_string(),
			},
		);
		assert_eq!(adapted, Some(ReasoningStrategy::ChainOfThought));
	}

	#[test]
	fn performance_tracking() {
		let mut selector = StrategySelector::new();

		// Record some attempts
		selector.record_attempt(
			TaskType::BugFix,
			ReasoningStrategy::VerifiedReasoning,
			true,
			5000,
			1000,
		);
		selector.record_attempt(
			TaskType::BugFix,
			ReasoningStrategy::VerifiedReasoning,
			true,
			4000,
			900,
		);
		selector.record_attempt(
			TaskType::BugFix,
			ReasoningStrategy::VerifiedReasoning,
			false,
			6000,
			1100,
		);

		let stats = selector
			.get_stats(&TaskType::BugFix, &ReasoningStrategy::VerifiedReasoning)
			.unwrap();
		assert_eq!(stats.attempts, 3);
		assert_eq!(stats.successes, 2);
		assert!((stats.success_rate() - 0.666).abs() < 0.01);
	}
}
