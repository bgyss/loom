// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Feedback integration from verifications to memory.

use std::sync::Arc;

use loom_context_core::{
	FeedbackSummary, MemoryContent, MemoryId, MemoryItem, MemorySource, MemoryType,
	VerificationResult,
};

use crate::error::Result;
use crate::store::MemoryStore;

/// Integrates verification feedback into the memory system.
pub struct FeedbackIntegrator {
	/// Memory store reference.
	memory: Arc<MemoryStore>,
}

impl FeedbackIntegrator {
	/// Creates a new feedback integrator.
	pub fn new(memory: Arc<MemoryStore>) -> Self {
		Self { memory }
	}

	/// Processes verification results and updates memory.
	pub async fn integrate(&self, results: &[VerificationResult]) -> Result<FeedbackSummary> {
		let mut summary = FeedbackSummary::default();

		for result in results {
			if result.passed {
				summary.passed += 1;

				// Record successful pattern if we can extract a learning
				if let Some(learning) = self.extract_success_learning(result) {
					self.memory.add(learning).await?;
					summary.learnings_added += 1;
				}
			} else {
				summary.failed += 1;

				// Record error pattern for future reference
				let error_memory = self.create_error_memory(result);
				self.memory.add(error_memory).await?;
				summary.errors_recorded += 1;
			}
		}

		Ok(summary)
	}

	/// Extracts a learning from a successful verification.
	fn extract_success_learning(&self, result: &VerificationResult) -> Option<MemoryItem> {
		// Only create learnings for significant successes
		if result.duration.as_millis() < 100 {
			// Quick verifications aren't worth recording
			return None;
		}

		let content = MemoryContent::InteractionLearning {
			situation: format!(
				"Verification of '{}' (type: {})",
				result.claim.assertion,
				result.claim.claim_type.type_key()
			),
			approach: result.claim.evidence.join("; "),
			outcome: "Passed".to_string(),
			lesson: format!(
				"Successful {} verification took {:?}",
				result.claim.claim_type.type_key(),
				result.duration
			),
		};

		let item = MemoryItem::new(
			result.org_id,
			MemoryType::ShortTerm,
			content,
			MemorySource::Verification {
				verification_id: result.id.to_string(),
			},
		)
		.with_tags(vec!["success".to_string(), "verification".to_string()])
		.with_relevance(0.6);

		Some(item)
	}

	/// Creates an error memory from a failed verification.
	fn create_error_memory(&self, result: &VerificationResult) -> MemoryItem {
		let content = MemoryContent::ErrorResolution {
			error_pattern: format!(
				"Failed: {} ({})",
				result.claim.assertion,
				result.claim.claim_type.type_key()
			),
			root_cause: self.analyze_failure(&result.output),
			solution: String::new(), // To be filled when resolved
		};

		MemoryItem::new(
			result.org_id,
			MemoryType::ShortTerm,
			content,
			MemorySource::Verification {
				verification_id: result.id.to_string(),
			},
		)
		.with_tags(vec![
			"error".to_string(),
			"unresolved".to_string(),
			"verification".to_string(),
		])
		.with_relevance(0.8)
	}

	/// Analyzes failure output to extract root cause.
	fn analyze_failure(&self, output: &str) -> String {
		// Simple heuristic analysis
		let output_lower = output.to_lowercase();

		if output_lower.contains("not found") {
			"Resource not found".to_string()
		} else if output_lower.contains("permission") || output_lower.contains("denied") {
			"Permission/access issue".to_string()
		} else if output_lower.contains("timeout") {
			"Operation timed out".to_string()
		} else if output_lower.contains("error") {
			// Extract first line after "error"
			if let Some(pos) = output_lower.find("error") {
				let rest = &output[pos..];
				let line = rest.lines().next().unwrap_or(rest);
				line.chars().take(100).collect()
			} else {
				"Unknown error".to_string()
			}
		} else {
			// Use first line of output
			output
				.lines()
				.next()
				.unwrap_or("Unknown failure")
				.chars()
				.take(100)
				.collect()
		}
	}

	/// Records a resolution for a previously failed verification.
	pub async fn record_resolution(&self, error_id: &MemoryId, solution: &str) -> Result<()> {
		// Get the memory item
		if let Some(mut item) = self.memory.get(error_id).await {
			// Update the solution in the content
			if let MemoryContent::ErrorResolution {
				error_pattern,
				root_cause,
				..
			} = item.content
			{
				item.content = MemoryContent::ErrorResolution {
					error_pattern,
					root_cause,
					solution: solution.to_string(),
				};

				// Update tags
				item.tags.retain(|t| t != "unresolved");
				item.tags.push("resolved".to_string());

				// This is valuable knowledge - consider promotion
				item.relevance_score = 1.0;
			}

			// Re-add (effectively update) the item
			self.memory.add(item).await?;
		}

		Ok(())
	}

	/// Returns unresolved error memories.
	pub async fn get_unresolved_errors(&self) -> Vec<MemoryItem> {
		self.memory.get_by_tag("unresolved").await
	}

	/// Promotes an error resolution to long-term memory.
	pub async fn promote_resolution(&self, error_id: &MemoryId) -> Result<()> {
		self.memory.promote(error_id).await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_context_core::{Claim, ClaimType, OrgId};
	use std::time::Duration;

	fn make_success_result(org_id: OrgId) -> VerificationResult {
		let claim = Claim::new(
			"tests pass",
			ClaimType::TestsPass {
				test_pattern: "test_*".to_string(),
			},
		)
		.with_evidence("ran cargo test");

		VerificationResult::success(org_id, claim, "10 tests passed", Duration::from_millis(500))
	}

	fn make_failure_result(org_id: OrgId) -> VerificationResult {
		let claim = Claim::new(
			"command succeeds",
			ClaimType::CommandSucceeds {
				command: "cargo build".to_string(),
			},
		);

		VerificationResult::failure(
			org_id,
			claim,
			"error: could not compile `foo`\nsome additional context",
			Duration::from_millis(200),
		)
	}

	#[tokio::test]
	async fn integrate_success() {
		let org_id = OrgId::new();
		let store = Arc::new(MemoryStore::with_defaults(org_id));
		let integrator = FeedbackIntegrator::new(store.clone());

		let results = vec![make_success_result(org_id)];
		let summary = integrator.integrate(&results).await.unwrap();

		assert_eq!(summary.passed, 1);
		assert_eq!(summary.failed, 0);
		assert!(summary.learnings_added > 0);
	}

	#[tokio::test]
	async fn integrate_failure() {
		let org_id = OrgId::new();
		let store = Arc::new(MemoryStore::with_defaults(org_id));
		let integrator = FeedbackIntegrator::new(store.clone());

		let results = vec![make_failure_result(org_id)];
		let summary = integrator.integrate(&results).await.unwrap();

		assert_eq!(summary.passed, 0);
		assert_eq!(summary.failed, 1);
		assert_eq!(summary.errors_recorded, 1);

		// Should have unresolved error
		let unresolved = integrator.get_unresolved_errors().await;
		assert!(!unresolved.is_empty());
	}

	#[tokio::test]
	async fn record_and_promote_resolution() {
		let org_id = OrgId::new();
		let store = Arc::new(MemoryStore::with_defaults(org_id));
		let integrator = FeedbackIntegrator::new(store.clone());

		// First create an error
		let results = vec![make_failure_result(org_id)];
		integrator.integrate(&results).await.unwrap();

		// Get the error
		let unresolved = integrator.get_unresolved_errors().await;
		assert!(!unresolved.is_empty());
		let error_id = unresolved[0].id;

		// Record resolution
		integrator
			.record_resolution(&error_id, "Fixed by adding dependency")
			.await
			.unwrap();

		// Should no longer be unresolved
		let still_unresolved = integrator.get_unresolved_errors().await;
		assert!(still_unresolved.is_empty());
	}

	#[test]
	fn analyze_failure_patterns() {
		let org_id = OrgId::new();
		let store = Arc::new(MemoryStore::with_defaults(org_id));
		let integrator = FeedbackIntegrator::new(store);

		assert!(integrator
			.analyze_failure("file not found: foo.rs")
			.contains("not found"));
		assert!(integrator
			.analyze_failure("permission denied")
			.contains("Permission"));
		assert!(integrator
			.analyze_failure("connection timeout")
			.contains("timed out"));
	}

	use crate::store::MemoryStore;
}
