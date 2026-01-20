// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Context manager combining all context management components.

use std::sync::Arc;

use loom_context_core::{
	AssembledContext, Claim, DriftDetectorConfig, DriftSignal, FeedbackSummary,
	MemoryId, MemoryItem, MemoryQuery, OrgId, ReasoningStrategy, ResourceBudget, ResourceUsage,
	StrategyFeedback, TaskType, VerificationResult,
};

use crate::assembler::ContextAssembler;
use crate::drift_corrector::{CorrectionReport, DriftCorrector};
use crate::drift_detector::{AgentAction, DriftDetector};
use crate::error::Result;
use crate::feedback::FeedbackIntegrator;
use crate::store::{DecayConfig, MemoryStore};
use crate::strategy::StrategySelector;
use crate::verification::{create_default_engine, VerificationEngine};

/// Main facade for context management operations.
pub struct ContextManager {
	/// Memory store.
	memory: Arc<MemoryStore>,
	/// Context assembler.
	assembler: ContextAssembler,
	/// Drift detector.
	drift_detector: DriftDetector,
	/// Drift corrector.
	drift_corrector: DriftCorrector,
	/// Verification engine.
	verification_engine: VerificationEngine,
	/// Feedback integrator.
	feedback_integrator: FeedbackIntegrator,
	/// Strategy selector.
	strategy_selector: StrategySelector,
	/// Resource budget.
	budget: ResourceBudget,
	/// Organization ID.
	org_id: OrgId,
}

impl ContextManager {
	/// Creates a new context manager with default configuration.
	pub fn new(org_id: OrgId) -> Self {
		let budget = ResourceBudget::default();
		let memory = Arc::new(MemoryStore::new(org_id, budget.clone(), DecayConfig::default()));

		Self {
			assembler: ContextAssembler::new(memory.clone(), budget.clone()),
			drift_detector: DriftDetector::new(DriftDetectorConfig::default()),
			drift_corrector: DriftCorrector::new(memory.clone()),
			verification_engine: create_default_engine(org_id),
			feedback_integrator: FeedbackIntegrator::new(memory.clone()),
			strategy_selector: StrategySelector::with_budget(budget.clone()),
			memory,
			budget,
			org_id,
		}
	}

	/// Creates a context manager with custom configuration.
	pub fn with_config(
		org_id: OrgId,
		budget: ResourceBudget,
		decay_config: DecayConfig,
		drift_config: DriftDetectorConfig,
	) -> Self {
		let memory = Arc::new(MemoryStore::new(org_id, budget.clone(), decay_config));

		Self {
			assembler: ContextAssembler::new(memory.clone(), budget.clone()),
			drift_detector: DriftDetector::new(drift_config),
			drift_corrector: DriftCorrector::new(memory.clone()),
			verification_engine: create_default_engine(org_id),
			feedback_integrator: FeedbackIntegrator::new(memory.clone()),
			strategy_selector: StrategySelector::with_budget(budget.clone()),
			memory,
			budget,
			org_id,
		}
	}

	/// Returns the organization ID.
	pub fn org_id(&self) -> OrgId {
		self.org_id
	}

	/// Returns the resource budget.
	pub fn budget(&self) -> &ResourceBudget {
		&self.budget
	}

	/// Returns the memory store.
	pub fn memory(&self) -> Arc<MemoryStore> {
		self.memory.clone()
	}

	// === Memory Operations ===

	/// Adds a memory item.
	pub async fn add_memory(&self, item: MemoryItem) -> Result<MemoryId> {
		self.memory.add(item).await
	}

	/// Retrieves a memory item by ID.
	pub async fn get_memory(&self, id: &MemoryId) -> Option<MemoryItem> {
		self.memory.get(id).await
	}

	/// Retrieves relevant memories for a query.
	pub async fn query_memories(&self, query: &MemoryQuery) -> Vec<MemoryItem> {
		self.memory.retrieve(query).await
	}

	/// Promotes a memory to long-term storage.
	pub async fn promote_memory(&self, id: &MemoryId) -> Result<()> {
		self.memory.promote(id).await
	}

	/// Removes a memory item.
	pub async fn remove_memory(&self, id: &MemoryId) -> Result<MemoryItem> {
		self.memory.remove(id).await
	}

	// === Context Assembly ===

	/// Assembles context for a task.
	pub async fn assemble(
		&self,
		task_id: &str,
		task_content: &str,
		usage: &ResourceUsage,
	) -> Result<AssembledContext> {
		self.assembler.assemble(task_id, task_content, usage).await
	}

	/// Compresses context when near limits.
	pub async fn compress(&self, context: &AssembledContext) -> Result<AssembledContext> {
		self.assembler.compress(context).await
	}

	// === Drift Detection and Correction ===

	/// Records an agent action for drift detection.
	pub fn record_action(&mut self, action: AgentAction) {
		self.drift_detector.record_action(action);
	}

	/// Records a verification result for drift detection.
	pub fn record_verification_for_drift(&mut self, result: VerificationResult) {
		self.drift_detector.record_verification(result);
	}

	/// Detects drift signals.
	pub fn detect_drift(&self) -> Vec<DriftSignal> {
		self.drift_detector.detect()
	}

	/// Corrects detected drift.
	pub async fn correct_drift(&self, signals: &[DriftSignal]) -> Result<CorrectionReport> {
		self.drift_corrector.correct(signals).await
	}

	/// Suggests a prompt injection for a drift signal.
	pub fn suggest_drift_prompt(&self, signal: &DriftSignal) -> Option<String> {
		self.drift_corrector.suggest_prompt_injection(signal)
	}

	/// Marks context as refreshed.
	pub fn mark_context_refreshed(&mut self) {
		self.drift_detector.mark_context_refreshed();
	}

	/// Refreshes context (applies decay, prunes, etc.).
	pub async fn refresh(&self) -> Result<()> {
		self.memory.apply_decay().await;
		self.memory.prune_to_budget().await?;
		Ok(())
	}

	// === Verification ===

	/// Verifies a single claim.
	pub async fn verify(&self, claim: &Claim) -> Result<VerificationResult> {
		self.verification_engine.verify(claim).await
	}

	/// Verifies multiple claims.
	pub async fn verify_claims(&self, claims: &[Claim]) -> Vec<VerificationResult> {
		self.verification_engine.verify_claims(claims).await
	}

	// === Feedback Integration ===

	/// Integrates verification feedback into memory.
	pub async fn integrate_feedback(&self, results: &[VerificationResult]) -> Result<FeedbackSummary> {
		self.feedback_integrator.integrate(results).await
	}

	/// Absorbs learnings from a task completion.
	pub async fn absorb_learnings(&self, results: &[VerificationResult]) -> Result<FeedbackSummary> {
		// Integrate feedback
		let summary = self.feedback_integrator.integrate(results).await?;

		// Apply decay to existing memories
		self.memory.apply_decay().await;

		Ok(summary)
	}

	/// Records a resolution for a failed verification.
	pub async fn record_resolution(&self, error_id: &MemoryId, solution: &str) -> Result<()> {
		self.feedback_integrator
			.record_resolution(error_id, solution)
			.await
	}

	/// Returns unresolved errors.
	pub async fn get_unresolved_errors(&self) -> Vec<MemoryItem> {
		self.feedback_integrator.get_unresolved_errors().await
	}

	// === Strategy Selection ===

	/// Selects a reasoning strategy for a task.
	pub fn select_strategy(
		&self,
		task_description: &str,
		resources: &ResourceUsage,
	) -> ReasoningStrategy {
		self.strategy_selector.select(task_description, resources)
	}

	/// Adapts strategy based on feedback.
	pub fn adapt_strategy(
		&self,
		current: &ReasoningStrategy,
		feedback: &StrategyFeedback,
	) -> Option<ReasoningStrategy> {
		self.strategy_selector.adapt(current, feedback)
	}

	/// Records a strategy attempt outcome.
	pub fn record_strategy_attempt(
		&mut self,
		task_type: TaskType,
		strategy: ReasoningStrategy,
		success: bool,
		duration_ms: u64,
		tokens_used: u64,
	) {
		self.strategy_selector
			.record_attempt(task_type, strategy, success, duration_ms, tokens_used);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_context_core::{ClaimType, MemoryContent, MemorySource, MemoryType};

	#[tokio::test]
	async fn context_manager_memory_operations() {
		let org_id = OrgId::new();
		let manager = ContextManager::new(org_id);

		// Add a memory
		let content = MemoryContent::CodePattern {
			pattern: "error handling".to_string(),
			example: "Result<T, E>".to_string(),
			applicability: "fallible operations".to_string(),
		};
		let item = MemoryItem::new(org_id, MemoryType::ShortTerm, content, MemorySource::Manual);
		let id = item.id;

		manager.add_memory(item).await.unwrap();

		// Retrieve it
		let retrieved = manager.get_memory(&id).await;
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().id, id);
	}

	#[tokio::test]
	async fn context_manager_assembly() {
		let org_id = OrgId::new();
		let manager = ContextManager::new(org_id);

		let usage = ResourceUsage::new();
		let context = manager
			.assemble("task-1", "Implement error handling", &usage)
			.await
			.unwrap();

		assert!(!context.sections.is_empty());
	}

	#[tokio::test]
	async fn context_manager_verification() {
		let org_id = OrgId::new();
		let manager = ContextManager::new(org_id);

		let claim = Claim::new(
			"tests pass",
			ClaimType::TestsPass {
				test_pattern: "test_*".to_string(),
			},
		);

		let result = manager.verify(&claim).await.unwrap();
		assert!(result.passed);
	}

	#[test]
	fn context_manager_strategy_selection() {
		let org_id = OrgId::new();
		let manager = ContextManager::new(org_id);

		let usage = ResourceUsage::new();
		let strategy = manager.select_strategy("fix the authentication bug", &usage);

		// Bug fixes should use verified reasoning by default
		assert_eq!(strategy, ReasoningStrategy::VerifiedReasoning);
	}

	#[test]
	fn context_manager_drift_detection() {
		let org_id = OrgId::new();
		let mut manager = ContextManager::new(org_id);

		// Record some repetitive actions
		for _ in 0..6 {
			manager.record_action(AgentAction::new("edit").with_topics(vec!["same_file"]));
		}

		let signals = manager.detect_drift();
		// Should detect circular reasoning
		assert!(signals.iter().any(|s| matches!(s, DriftSignal::CircularReasoning { .. })));
	}

	#[tokio::test]
	async fn context_manager_full_workflow() {
		let org_id = OrgId::new();
		let mut manager = ContextManager::new(org_id);

		// 1. Select strategy
		let usage = ResourceUsage::new();
		let strategy = manager.select_strategy("add a new feature", &usage);
		assert_eq!(strategy, ReasoningStrategy::StepwiseRefinement);

		// 2. Assemble context
		let context = manager
			.assemble("task-1", "Add user authentication", &usage)
			.await
			.unwrap();
		assert!(!context.sections.is_empty());

		// 3. Verify claims (using a command that succeeds)
		let claims = vec![Claim::new(
			"command runs",
			ClaimType::CommandSucceeds {
				command: "echo 'test'".to_string(),
			},
		)];
		let results = manager.verify_claims(&claims).await;
		assert!(!results.is_empty());
		assert!(results[0].passed);

		// 4. Integrate feedback
		let summary = manager.absorb_learnings(&results).await.unwrap();
		assert_eq!(summary.passed, 1);

		// 5. Record action
		manager.record_action(AgentAction::new("implement").with_topics(vec!["auth"]));

		// 6. Check for drift - filter out stale context since timing can vary
		let signals = manager.detect_drift();
		// With a single action and single successful verification, we shouldn't detect
		// high failure rate, circular reasoning, tunnel vision, or risk aversion
		let critical_signals: Vec<_> = signals
			.into_iter()
			.filter(|s| {
				matches!(
					s,
					DriftSignal::HighFailureRate { .. }
						| DriftSignal::CircularReasoning { .. }
						| DriftSignal::Contradiction { .. }
				)
			})
			.collect();
		assert!(critical_signals.is_empty());
	}
}
