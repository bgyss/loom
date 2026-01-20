// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Persistent strategy selection with database-backed learning.

use std::sync::Arc;

use async_trait::async_trait;
use loom_context_core::{
	OrgId, PerformanceStats, ReasoningStrategy, ResourceBudget, ResourceUsage, StrategyFeedback,
	TaskType,
};
use tokio::sync::RwLock;

use crate::error::Result;
use crate::strategy::{StrategyFeedbackMonitor, StrategyPromptBuilder, TaskClassifier};

/// Trait for persistent strategy performance storage.
#[async_trait]
pub trait StrategyPerformanceStore: Send + Sync {
	/// Records a strategy attempt outcome.
	async fn record_attempt(
		&self,
		org_id: &OrgId,
		task_type: TaskType,
		strategy: ReasoningStrategy,
		success: bool,
		duration_ms: u64,
		tokens_used: u64,
	) -> Result<()>;

	/// Gets performance stats for a specific strategy and task type.
	async fn get_stats(
		&self,
		org_id: &OrgId,
		task_type: &TaskType,
		strategy: &ReasoningStrategy,
	) -> Result<Option<PerformanceStats>>;

	/// Gets all strategy stats for a task type.
	async fn get_all_stats_for_task_type(
		&self,
		org_id: &OrgId,
		task_type: &TaskType,
	) -> Result<Vec<(ReasoningStrategy, PerformanceStats)>>;
}

/// A strategy selector that learns from historical performance stored in a database.
pub struct PersistentStrategySelector {
	/// Organization ID.
	org_id: OrgId,
	/// Persistence store.
	store: Arc<dyn StrategyPerformanceStore>,
	/// Task classifier.
	classifier: TaskClassifier,
	/// Prompt builder.
	prompt_builder: StrategyPromptBuilder,
	/// Feedback monitor.
	feedback_monitor: Arc<RwLock<StrategyFeedbackMonitor>>,
	/// Resource budget.
	budget: ResourceBudget,
	/// Minimum attempts before trusting historical data.
	min_attempts_for_history: u32,
	/// Minimum success rate to consider a strategy viable.
	min_success_rate: f64,
}

impl PersistentStrategySelector {
	/// Creates a new persistent strategy selector.
	pub fn new(org_id: OrgId, store: Arc<dyn StrategyPerformanceStore>) -> Self {
		Self {
			org_id,
			store,
			classifier: TaskClassifier::new(),
			prompt_builder: StrategyPromptBuilder::new(),
			feedback_monitor: Arc::new(RwLock::new(StrategyFeedbackMonitor::new())),
			budget: ResourceBudget::default(),
			min_attempts_for_history: 3,
			min_success_rate: 0.5,
		}
	}

	/// Sets the resource budget.
	pub fn with_budget(mut self, budget: ResourceBudget) -> Self {
		self.budget = budget;
		self
	}

	/// Sets the minimum attempts required before using historical data.
	pub fn with_min_attempts(mut self, min_attempts: u32) -> Self {
		self.min_attempts_for_history = min_attempts;
		self
	}

	/// Sets the minimum success rate for a strategy to be considered viable.
	pub fn with_min_success_rate(mut self, rate: f64) -> Self {
		self.min_success_rate = rate;
		self
	}

	/// Classifies a task description into a task type.
	pub fn classify_task(&self, description: &str) -> TaskType {
		self.classifier.classify(description)
	}

	/// Selects a strategy based on task and resources, using historical performance.
	pub async fn select(
		&self,
		task_description: &str,
		resources: &ResourceUsage,
	) -> ReasoningStrategy {
		let task_type = self.classifier.classify(task_description);

		// Check if resources are constrained
		let resource_pressure = resources.is_near_limit(&self.budget, 0.7);
		if resource_pressure.any() {
			return ReasoningStrategy::ChainOfThought;
		}

		// Try to use historical performance data
		if let Some(best) = self.best_strategy_from_history(&task_type).await {
			return best;
		}

		// Fall back to default for task type
		task_type.default_strategy()
	}

	/// Finds the best strategy for a task type based on historical performance.
	async fn best_strategy_from_history(&self, task_type: &TaskType) -> Option<ReasoningStrategy> {
		let all_stats = self
			.store
			.get_all_stats_for_task_type(&self.org_id, task_type)
			.await
			.ok()?;

		let mut best: Option<(ReasoningStrategy, f64)> = None;

		for (strategy, stats) in all_stats {
			if stats.attempts >= self.min_attempts_for_history {
				let rate = stats.success_rate();
				if rate >= self.min_success_rate {
					if best.is_none() || rate > best.unwrap().1 {
						best = Some((strategy, rate));
					}
				}
			}
		}

		best.map(|(s, _)| s)
	}

	/// Adapts strategy based on feedback signals.
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
			(
				ReasoningStrategy::ParallelExploration { .. },
				StrategyFeedback::ResourcePressure { .. },
			)
			| (ReasoningStrategy::VerifiedReasoning, StrategyFeedback::ResourcePressure { .. }) => {
				Some(ReasoningStrategy::ChainOfThought)
			}

			// High complexity suggests verified reasoning
			(_, StrategyFeedback::HighComplexity { complexity_score }) if *complexity_score > 0.8 => {
				Some(ReasoningStrategy::VerifiedReasoning)
			}

			// Recovery suggests current strategy is working
			(_, StrategyFeedback::RecoveredFromFailure) => None,

			_ => None,
		}
	}

	/// Records a strategy attempt outcome.
	pub async fn record_attempt(
		&self,
		task_type: TaskType,
		strategy: ReasoningStrategy,
		success: bool,
		duration_ms: u64,
		tokens_used: u64,
	) -> Result<()> {
		self.store
			.record_attempt(&self.org_id, task_type, strategy, success, duration_ms, tokens_used)
			.await
	}

	/// Records a verification result for feedback monitoring.
	pub async fn record_verification(&self, passed: bool) {
		let mut monitor = self.feedback_monitor.write().await;
		monitor.record_verification(passed);
	}

	/// Records an action for feedback monitoring.
	pub async fn record_action(&self, topics: Vec<String>) {
		let mut monitor = self.feedback_monitor.write().await;
		monitor.record_action(topics);
	}

	/// Records progress (resets no-progress counter).
	pub async fn record_progress(&self) {
		let mut monitor = self.feedback_monitor.write().await;
		monitor.record_progress();
	}

	/// Checks for feedback signals based on current monitoring state.
	pub async fn check_feedback(&self, resources: &ResourceUsage) -> Vec<StrategyFeedback> {
		let monitor = self.feedback_monitor.read().await;
		monitor.check(resources, &self.budget)
	}

	/// Resets the feedback monitor.
	pub async fn reset_monitor(&self) {
		let mut monitor = self.feedback_monitor.write().await;
		monitor.reset();
	}

	/// Returns the prompt builder.
	pub fn prompt_builder(&self) -> &StrategyPromptBuilder {
		&self.prompt_builder
	}

	/// Builds a strategy prompt section.
	pub fn build_strategy_prompt(&self, strategy: &ReasoningStrategy) -> String {
		self.prompt_builder.build_strategy_section(strategy)
	}

	/// Builds an adaptation notice.
	pub fn build_adaptation_notice(
		&self,
		old: &ReasoningStrategy,
		new: &ReasoningStrategy,
		feedback: &StrategyFeedback,
	) -> String {
		self.prompt_builder.build_adaptation_notice(old, new, feedback)
	}

	/// Gets performance stats for a specific strategy.
	pub async fn get_stats(
		&self,
		task_type: &TaskType,
		strategy: &ReasoningStrategy,
	) -> Result<Option<PerformanceStats>> {
		self.store.get_stats(&self.org_id, task_type, strategy).await
	}

	/// Gets all strategy stats for a task type.
	pub async fn get_all_stats(
		&self,
		task_type: &TaskType,
	) -> Result<Vec<(ReasoningStrategy, PerformanceStats)>> {
		self.store.get_all_stats_for_task_type(&self.org_id, task_type).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;
	use std::sync::Mutex;

	/// In-memory implementation of StrategyPerformanceStore for testing.
	struct InMemoryStrategyStore {
		data: Mutex<HashMap<(String, String, String), PerformanceStats>>,
	}

	impl InMemoryStrategyStore {
		fn new() -> Self {
			Self {
				data: Mutex::new(HashMap::new()),
			}
		}
	}

	#[async_trait]
	impl StrategyPerformanceStore for InMemoryStrategyStore {
		async fn record_attempt(
			&self,
			org_id: &OrgId,
			task_type: TaskType,
			strategy: ReasoningStrategy,
			success: bool,
			duration_ms: u64,
			tokens_used: u64,
		) -> Result<()> {
			let key = (org_id.to_string(), task_type.name().to_string(), strategy.name().to_string());
			let mut data = self.data.lock().unwrap();
			let stats = data.entry(key).or_insert_with(PerformanceStats::new);
			stats.record(success, duration_ms, tokens_used);
			Ok(())
		}

		async fn get_stats(
			&self,
			org_id: &OrgId,
			task_type: &TaskType,
			strategy: &ReasoningStrategy,
		) -> Result<Option<PerformanceStats>> {
			let key = (org_id.to_string(), task_type.name().to_string(), strategy.name().to_string());
			let data = self.data.lock().unwrap();
			Ok(data.get(&key).cloned())
		}

		async fn get_all_stats_for_task_type(
			&self,
			org_id: &OrgId,
			task_type: &TaskType,
		) -> Result<Vec<(ReasoningStrategy, PerformanceStats)>> {
			let prefix = (org_id.to_string(), task_type.name().to_string());
			let data = self.data.lock().unwrap();

			let mut results = Vec::new();
			for ((o, t, s), stats) in data.iter() {
				if *o == prefix.0 && *t == prefix.1 {
					if let Some(strategy) = match s.as_str() {
						"chain_of_thought" => Some(ReasoningStrategy::ChainOfThought),
						"program_of_thought" => Some(ReasoningStrategy::ProgramOfThought),
						"stepwise_refinement" => Some(ReasoningStrategy::StepwiseRefinement),
						"parallel_exploration" => Some(ReasoningStrategy::ParallelExploration { branches: 2 }),
						"verified_reasoning" => Some(ReasoningStrategy::VerifiedReasoning),
						_ => None,
					} {
						results.push((strategy, stats.clone()));
					}
				}
			}
			Ok(results)
		}
	}

	#[tokio::test]
	async fn select_uses_default_without_history() {
		let org_id = OrgId::new();
		let store = Arc::new(InMemoryStrategyStore::new());
		let selector = PersistentStrategySelector::new(org_id, store);

		let usage = ResourceUsage::new();
		let strategy = selector.select("fix the login bug", &usage).await;

		// Bug fixes default to verified reasoning
		assert_eq!(strategy, ReasoningStrategy::VerifiedReasoning);
	}

	#[tokio::test]
	async fn select_uses_history_when_available() {
		let org_id = OrgId::new();
		let store = Arc::new(InMemoryStrategyStore::new());

		// Record successful attempts with ChainOfThought for bug fixes
		for _ in 0..5 {
			store
				.record_attempt(
					&org_id,
					TaskType::BugFix,
					ReasoningStrategy::ChainOfThought,
					true,
					1000,
					500,
				)
				.await
				.unwrap();
		}

		let selector = PersistentStrategySelector::new(org_id, store);
		let usage = ResourceUsage::new();
		let strategy = selector.select("fix the login bug", &usage).await;

		// Should use ChainOfThought based on history
		assert_eq!(strategy, ReasoningStrategy::ChainOfThought);
	}

	#[tokio::test]
	async fn select_falls_back_under_resource_pressure() {
		let org_id = OrgId::new();
		let store = Arc::new(InMemoryStrategyStore::new());
		let selector = PersistentStrategySelector::new(org_id, store);

		let mut usage = ResourceUsage::new();
		usage.context_tokens = 100_000; // High usage

		let strategy = selector.select("add a new feature", &usage).await;

		// Should use simple strategy under pressure
		assert_eq!(strategy, ReasoningStrategy::ChainOfThought);
	}

	#[tokio::test]
	async fn adaptation_on_high_failure_rate() {
		let org_id = OrgId::new();
		let store = Arc::new(InMemoryStrategyStore::new());
		let selector = PersistentStrategySelector::new(org_id, store);

		let adapted = selector.adapt(
			&ReasoningStrategy::StepwiseRefinement,
			&StrategyFeedback::HighFailureRate { rate: 0.7 },
		);

		assert_eq!(adapted, Some(ReasoningStrategy::VerifiedReasoning));
	}

	#[tokio::test]
	async fn feedback_monitoring() {
		let org_id = OrgId::new();
		let store = Arc::new(InMemoryStrategyStore::new());
		let selector = PersistentStrategySelector::new(org_id, store);

		// Record failures
		selector.record_verification(false).await;
		selector.record_verification(false).await;
		selector.record_verification(false).await;
		selector.record_verification(true).await;

		let feedback = selector.check_feedback(&ResourceUsage::new()).await;
		assert!(feedback.iter().any(|f| matches!(f, StrategyFeedback::HighFailureRate { .. })));
	}

	#[tokio::test]
	async fn prompt_building() {
		let org_id = OrgId::new();
		let store = Arc::new(InMemoryStrategyStore::new());
		let selector = PersistentStrategySelector::new(org_id, store);

		let prompt = selector.build_strategy_prompt(&ReasoningStrategy::VerifiedReasoning);
		assert!(prompt.contains("Reasoning Strategy"));
		assert!(prompt.contains("verified_reasoning"));
		assert!(prompt.contains("Guidelines"));
	}
}
