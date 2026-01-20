// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Drift correction strategies.

use std::collections::HashMap;
use std::sync::Arc;

use loom_context_core::{CorrectionAction, DriftSignal};

use crate::error::Result;
use crate::store::MemoryStore;

/// Report of corrections applied to address drift.
#[derive(Debug, Clone, Default)]
pub struct CorrectionReport {
	/// Actions taken.
	pub actions: Vec<CorrectionAction>,
	/// Whether correction was successful.
	pub success: bool,
	/// Additional notes.
	pub notes: Vec<String>,
}

impl CorrectionReport {
	/// Creates a new correction report.
	pub fn new() -> Self {
		Self::default()
	}

	/// Adds an action to the report.
	pub fn add_action(&mut self, action: CorrectionAction) {
		self.actions.push(action);
	}

	/// Adds a note to the report.
	pub fn add_note(&mut self, note: impl Into<String>) {
		self.notes.push(note.into());
	}

	/// Marks the correction as successful.
	pub fn mark_success(&mut self) {
		self.success = true;
	}
}

/// Applies corrections based on detected drift signals.
pub struct DriftCorrector {
	/// Memory store reference (for future context-aware corrections).
	#[allow(dead_code)]
	memory: Arc<MemoryStore>,
}

impl DriftCorrector {
	/// Creates a new drift corrector.
	pub fn new(memory: Arc<MemoryStore>) -> Self {
		Self { memory }
	}

	/// Applies corrections for detected drift signals.
	pub async fn correct(&self, signals: &[DriftSignal]) -> Result<CorrectionReport> {
		let mut report = CorrectionReport::new();

		for signal in signals {
			match signal {
				DriftSignal::HighFailureRate { rate } => {
					report.add_note(format!("High failure rate detected: {:.1}%", rate * 100.0));
					report.add_action(CorrectionAction::RefreshedContext);
				}

				DriftSignal::RepeatedFailures { claim_type, count } => {
					report.add_note(format!(
						"Repeated failures on {}: {} times",
						claim_type, count
					));
					// Suggest trying a different approach
					report.add_action(CorrectionAction::FreshApproach {
						approach: format!(
							"Consider alternative verification strategy for {} claims",
							claim_type
						),
					});
				}

				DriftSignal::TunnelVision { topic_distribution } => {
					let neglected = self.identify_neglected_areas(topic_distribution);
					report.add_note(format!(
						"Tunnel vision detected. Neglected areas: {:?}",
						neglected
					));
					report.add_action(CorrectionAction::ForcedExploration { areas: neglected });
				}

				DriftSignal::RiskAversion {
					avg_change_size,
					threshold,
				} => {
					report.add_note(format!(
						"Risk aversion: avg change size {:.1} below threshold {:.1}",
						avg_change_size, threshold
					));
					report.add_action(CorrectionAction::PromptInjection {
						prompt: "Consider larger, more impactful changes. Small incremental \
								 changes may indicate tunnel vision or reluctance to make \
								 necessary modifications."
							.to_string(),
					});
				}

				DriftSignal::CircularReasoning { pattern } => {
					report.add_note(format!("Circular reasoning detected: {}", pattern));
					let fresh_approach = self.generate_fresh_approach(pattern);
					report.add_action(CorrectionAction::FreshApproach {
						approach: fresh_approach,
					});
				}

				DriftSignal::StaleContext { age } => {
					report.add_note(format!(
						"Context staleness: {:.1} minutes old",
						age.as_secs_f64() / 60.0
					));
					report.add_action(CorrectionAction::RefreshedContext);
				}

				DriftSignal::Contradiction { claim_a, claim_b } => {
					report.add_note(format!(
						"Contradiction detected between claims: '{}' vs '{}'",
						truncate(claim_a, 50),
						truncate(claim_b, 50)
					));
					let resolution = self.resolve_contradiction(claim_a, claim_b);
					report.add_action(CorrectionAction::ResolvedContradiction { resolution });
				}
			}
		}

		if !report.actions.is_empty() {
			report.mark_success();
		}

		Ok(report)
	}

	/// Identifies areas that have been neglected based on topic distribution.
	fn identify_neglected_areas(&self, distribution: &HashMap<String, f64>) -> Vec<String> {
		// Find topics with low representation
		let threshold = 0.1; // Less than 10%
		distribution
			.iter()
			.filter(|(_, &pct)| pct < threshold)
			.map(|(topic, _)| topic.clone())
			.collect()
	}

	/// Generates a fresh approach for breaking circular reasoning.
	fn generate_fresh_approach(&self, stuck_pattern: &str) -> String {
		format!(
			"The agent appears stuck in pattern: '{}'. Consider:\n\
			 1. Re-examine the original goal\n\
			 2. Try a completely different approach\n\
			 3. Break down the problem differently\n\
			 4. Check if assumptions are still valid",
			truncate(stuck_pattern, 100)
		)
	}

	/// Attempts to resolve a contradiction between claims.
	fn resolve_contradiction(&self, claim_a: &str, claim_b: &str) -> String {
		format!(
			"Contradiction detected:\n\
			 - Claim A: {}\n\
			 - Claim B: {}\n\n\
			 Resolution: Re-verify both claims independently. \
			 One or both may be based on stale information.",
			truncate(claim_a, 100),
			truncate(claim_b, 100)
		)
	}

	/// Suggests prompt injection for a given drift signal.
	pub fn suggest_prompt_injection(&self, signal: &DriftSignal) -> Option<String> {
		match signal {
			DriftSignal::HighFailureRate { rate } => Some(format!(
				"NOTICE: Verification failure rate is high ({:.0}%). \
				 Before proceeding, verify assumptions and check for fundamental issues.",
				rate * 100.0
			)),

			DriftSignal::TunnelVision { topic_distribution } => {
				let dominant = topic_distribution
					.iter()
					.max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
					.map(|(k, _)| k.as_str())
					.unwrap_or("unknown");
				Some(format!(
					"NOTICE: Focus has been heavily on '{}'. \
					 Consider other aspects of the task that may need attention.",
					dominant
				))
			}

			DriftSignal::RiskAversion { .. } => Some(
				"NOTICE: Recent changes have been small and incremental. \
				 If the task requires larger changes, don't hesitate to make them."
					.to_string(),
			),

			DriftSignal::CircularReasoning { pattern } => Some(format!(
				"NOTICE: Similar actions have been repeated: '{}'. \
				 Step back and consider a different approach.",
				truncate(pattern, 50)
			)),

			DriftSignal::StaleContext { age } => Some(format!(
				"NOTICE: Context is {:.0} minutes old. \
				 Verify that information is still current before proceeding.",
				age.as_secs_f64() / 60.0
			)),

			_ => None,
		}
	}
}

/// Truncates a string to a maximum length with ellipsis.
fn truncate(s: &str, max_len: usize) -> String {
	if s.len() <= max_len {
		s.to_string()
	} else {
		format!("{}...", &s[..max_len.saturating_sub(3)])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_context_core::OrgId;
	use std::time::Duration;

	#[tokio::test]
	async fn correct_high_failure_rate() {
		let org_id = OrgId::new();
		let memory = Arc::new(MemoryStore::with_defaults(org_id));
		let corrector = DriftCorrector::new(memory);

		let signals = vec![DriftSignal::HighFailureRate { rate: 0.75 }];
		let report = corrector.correct(&signals).await.unwrap();

		assert!(report.success);
		assert!(!report.actions.is_empty());
		assert!(report
			.actions
			.iter()
			.any(|a| matches!(a, CorrectionAction::RefreshedContext)));
	}

	#[tokio::test]
	async fn correct_tunnel_vision() {
		let org_id = OrgId::new();
		let memory = Arc::new(MemoryStore::with_defaults(org_id));
		let corrector = DriftCorrector::new(memory);

		let distribution = HashMap::from([
			("tests".to_string(), 0.8),
			("docs".to_string(), 0.05),
			("src".to_string(), 0.15),
		]);
		let signals = vec![DriftSignal::TunnelVision {
			topic_distribution: distribution,
		}];

		let report = corrector.correct(&signals).await.unwrap();

		assert!(report.success);
		assert!(report
			.actions
			.iter()
			.any(|a| matches!(a, CorrectionAction::ForcedExploration { .. })));
	}

	#[tokio::test]
	async fn correct_circular_reasoning() {
		let org_id = OrgId::new();
		let memory = Arc::new(MemoryStore::with_defaults(org_id));
		let corrector = DriftCorrector::new(memory);

		let signals = vec![DriftSignal::CircularReasoning {
			pattern: "edit-same_file".to_string(),
		}];

		let report = corrector.correct(&signals).await.unwrap();

		assert!(report.success);
		assert!(report
			.actions
			.iter()
			.any(|a| matches!(a, CorrectionAction::FreshApproach { .. })));
	}

	#[test]
	fn prompt_injection_suggestions() {
		let org_id = OrgId::new();
		let memory = Arc::new(MemoryStore::with_defaults(org_id));
		let corrector = DriftCorrector::new(memory);

		let signal = DriftSignal::HighFailureRate { rate: 0.8 };
		let prompt = corrector.suggest_prompt_injection(&signal);
		assert!(prompt.is_some());
		assert!(prompt.unwrap().contains("80%"));

		let signal = DriftSignal::StaleContext {
			age: Duration::from_secs(3600),
		};
		let prompt = corrector.suggest_prompt_injection(&signal);
		assert!(prompt.is_some());
		assert!(prompt.unwrap().contains("60"));
	}

	#[test]
	fn truncate_function() {
		assert_eq!(truncate("short", 10), "short");
		assert_eq!(truncate("this is a long string", 10), "this is...");
	}
}
