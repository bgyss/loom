// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Drift detection for context degradation.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;

use chrono::{DateTime, Utc};
use loom_context_core::{DriftDetectorConfig, DriftSignal, VerificationResult};

/// An action performed by an agent, used for pattern detection.
#[derive(Debug, Clone)]
pub struct AgentAction {
	/// Action type identifier.
	pub action_type: String,
	/// Topics involved in the action.
	pub topics: Vec<String>,
	/// Size of change (e.g., lines changed).
	pub change_size: Option<f64>,
	/// Timestamp.
	pub timestamp: DateTime<Utc>,
}

impl AgentAction {
	/// Creates a new agent action.
	pub fn new(action_type: impl Into<String>) -> Self {
		Self {
			action_type: action_type.into(),
			topics: Vec::new(),
			change_size: None,
			timestamp: Utc::now(),
		}
	}

	/// Adds topics to the action.
	pub fn with_topics(mut self, topics: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.topics = topics.into_iter().map(Into::into).collect();
		self
	}

	/// Sets the change size.
	pub fn with_change_size(mut self, size: f64) -> Self {
		self.change_size = Some(size);
		self
	}

	/// Returns a pattern signature for circular reasoning detection.
	pub fn pattern_signature(&self) -> String {
		format!(
			"{}-{}",
			self.action_type,
			self.topics.join(",")
		)
	}
}

/// Detects drift in agent behavior and context.
pub struct DriftDetector {
	/// Verification history.
	verification_history: VecDeque<VerificationResult>,
	/// Recent actions for pattern detection.
	recent_actions: VecDeque<AgentAction>,
	/// Configuration.
	config: DriftDetectorConfig,
	/// Last context refresh timestamp.
	last_context_refresh: DateTime<Utc>,
}

impl DriftDetector {
	/// Creates a new drift detector.
	pub fn new(config: DriftDetectorConfig) -> Self {
		Self {
			verification_history: VecDeque::new(),
			recent_actions: VecDeque::new(),
			config,
			last_context_refresh: Utc::now(),
		}
	}

	/// Creates a drift detector with default config.
	pub fn with_defaults() -> Self {
		Self::new(DriftDetectorConfig::default())
	}

	/// Records a verification result.
	pub fn record_verification(&mut self, result: VerificationResult) {
		self.verification_history.push_back(result);

		// Trim history if too large
		while self.verification_history.len() > 100 {
			self.verification_history.pop_front();
		}
	}

	/// Records an agent action.
	pub fn record_action(&mut self, action: AgentAction) {
		self.recent_actions.push_back(action);

		// Trim to configured size
		while self.recent_actions.len() > self.config.action_history_size {
			self.recent_actions.pop_front();
		}
	}

	/// Marks context as refreshed.
	pub fn mark_context_refreshed(&mut self) {
		self.last_context_refresh = Utc::now();
	}

	/// Runs comprehensive drift detection.
	pub fn detect(&self) -> Vec<DriftSignal> {
		let mut signals = Vec::new();

		// Check verification patterns
		if let Some(signal) = self.detect_verification_drift(Duration::from_secs(3600)) {
			signals.push(signal);
		}

		// Check for tunnel vision
		if let Some(signal) = self.detect_tunnel_vision() {
			signals.push(signal);
		}

		// Check for risk aversion
		if let Some(signal) = self.detect_risk_aversion() {
			signals.push(signal);
		}

		// Check for circular reasoning
		if let Some(signal) = self.detect_circular_reasoning() {
			signals.push(signal);
		}

		// Check context staleness
		if let Some(signal) = self.detect_staleness() {
			signals.push(signal);
		}

		signals
	}

	/// Detects high failure rate or repeated failures.
	fn detect_verification_drift(&self, window: Duration) -> Option<DriftSignal> {
		let cutoff = Utc::now() - chrono::Duration::from_std(window).ok()?;
		let recent: Vec<_> = self
			.verification_history
			.iter()
			.filter(|r| r.verified_at > cutoff)
			.collect();

		if recent.is_empty() {
			return None;
		}

		let failure_count = recent.iter().filter(|r| !r.passed).count();
		let failure_rate = failure_count as f64 / recent.len() as f64;

		if failure_rate > self.config.max_failure_rate {
			return Some(DriftSignal::HighFailureRate { rate: failure_rate });
		}

		// Check for repeated failures on same claim type
		let mut failure_counts: HashMap<&str, u32> = HashMap::new();
		for result in recent.iter().filter(|r| !r.passed) {
			let key = result.claim.claim_type.type_key();
			*failure_counts.entry(key).or_default() += 1;
		}

		for (claim_type, count) in failure_counts {
			if count > self.config.repeated_failures_threshold {
				return Some(DriftSignal::RepeatedFailures {
					claim_type: claim_type.to_string(),
					count,
				});
			}
		}

		None
	}

	/// Detects tunnel vision (narrow focus on single topic).
	fn detect_tunnel_vision(&self) -> Option<DriftSignal> {
		if self.recent_actions.is_empty() {
			return None;
		}

		let mut topic_counts: HashMap<String, u32> = HashMap::new();

		for action in &self.recent_actions {
			for topic in &action.topics {
				*topic_counts.entry(topic.clone()).or_default() += 1;
			}
		}

		let total: u32 = topic_counts.values().sum();
		if total == 0 {
			return None;
		}

		let distribution: HashMap<String, f64> = topic_counts
			.into_iter()
			.map(|(k, v)| (k, v as f64 / total as f64))
			.collect();

		// Check if any single topic dominates
		for (_, pct) in &distribution {
			if *pct > self.config.tunnel_vision_threshold {
				return Some(DriftSignal::TunnelVision {
					topic_distribution: distribution,
				});
			}
		}

		None
	}

	/// Detects risk aversion (only small, safe changes).
	fn detect_risk_aversion(&self) -> Option<DriftSignal> {
		let change_sizes: Vec<f64> = self
			.recent_actions
			.iter()
			.filter_map(|a| a.change_size)
			.collect();

		if change_sizes.is_empty() {
			return None;
		}

		let avg_size = change_sizes.iter().sum::<f64>() / change_sizes.len() as f64;

		if avg_size < self.config.risk_aversion_threshold {
			return Some(DriftSignal::RiskAversion {
				avg_change_size: avg_size,
				threshold: self.config.risk_aversion_threshold,
			});
		}

		None
	}

	/// Detects circular reasoning (repeated similar actions).
	fn detect_circular_reasoning(&self) -> Option<DriftSignal> {
		let mut action_patterns: HashMap<String, Vec<usize>> = HashMap::new();

		for (i, action) in self.recent_actions.iter().enumerate() {
			let pattern = action.pattern_signature();
			action_patterns.entry(pattern).or_default().push(i);
		}

		// Check for suspicious patterns (same action repeated close together)
		for (pattern, indices) in action_patterns {
			if indices.len() > 3 {
				// Check if indices are close together
				let gaps: Vec<usize> = indices.windows(2).map(|w| w[1] - w[0]).collect();
				if gaps.is_empty() {
					continue;
				}
				let avg_gap = gaps.iter().sum::<usize>() as f64 / gaps.len() as f64;

				if avg_gap < 5.0 {
					return Some(DriftSignal::CircularReasoning { pattern });
				}
			}
		}

		None
	}

	/// Detects context staleness.
	fn detect_staleness(&self) -> Option<DriftSignal> {
		let age = Utc::now() - self.last_context_refresh;
		let age_duration = age.to_std().ok()?;

		if age_duration > self.config.staleness_threshold {
			return Some(DriftSignal::StaleContext { age: age_duration });
		}

		None
	}

	/// Returns recent verification failure rate.
	pub fn recent_failure_rate(&self, window: Duration) -> f64 {
		let cutoff = Utc::now()
			- chrono::Duration::from_std(window).unwrap_or(chrono::Duration::hours(1));
		let recent: Vec<_> = self
			.verification_history
			.iter()
			.filter(|r| r.verified_at > cutoff)
			.collect();

		if recent.is_empty() {
			0.0
		} else {
			let failures = recent.iter().filter(|r| !r.passed).count();
			failures as f64 / recent.len() as f64
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_context_core::{Claim, ClaimType, OrgId};

	fn make_verification(passed: bool, claim_type: ClaimType) -> VerificationResult {
		let claim = Claim::new("test assertion", claim_type);
		if passed {
			VerificationResult::success(OrgId::new(), claim, "ok", Duration::from_millis(100))
		} else {
			VerificationResult::failure(OrgId::new(), claim, "failed", Duration::from_millis(100))
		}
	}

	#[test]
	fn detect_high_failure_rate() {
		let mut detector = DriftDetector::with_defaults();

		// Add mostly failures
		for _ in 0..8 {
			detector.record_verification(make_verification(
				false,
				ClaimType::CommandSucceeds {
					command: "test".to_string(),
				},
			));
		}
		for _ in 0..2 {
			detector.record_verification(make_verification(
				true,
				ClaimType::CommandSucceeds {
					command: "test".to_string(),
				},
			));
		}

		let signals = detector.detect();
		assert!(signals.iter().any(|s| matches!(s, DriftSignal::HighFailureRate { .. })));
	}

	#[test]
	fn detect_repeated_failures() {
		// Set high failure rate threshold so we trigger RepeatedFailures, not HighFailureRate
		let config = DriftDetectorConfig::default()
			.with_max_failure_rate(0.99)
			.with_repeated_failures_threshold(3);
		let mut detector = DriftDetector::new(config);

		// Add some passing verifications to avoid HighFailureRate
		for _ in 0..5 {
			detector.record_verification(make_verification(
				true,
				ClaimType::CommandSucceeds {
					command: "echo ok".to_string(),
				},
			));
		}

		// Add repeated failures of same type (4 failures > threshold of 3)
		for _ in 0..4 {
			detector.record_verification(make_verification(
				false,
				ClaimType::TestsPass {
					test_pattern: "test_*".to_string(),
				},
			));
		}

		let signals = detector.detect();
		assert!(signals.iter().any(|s| matches!(s, DriftSignal::RepeatedFailures { .. })));
	}

	#[test]
	fn detect_tunnel_vision() {
		let config = DriftDetectorConfig::default().with_tunnel_vision_threshold(0.6);
		let mut detector = DriftDetector::new(config);

		// Add actions heavily focused on one topic
		for _ in 0..10 {
			detector.record_action(AgentAction::new("edit").with_topics(vec!["tests"]));
		}
		for _ in 0..2 {
			detector.record_action(AgentAction::new("edit").with_topics(vec!["docs"]));
		}

		let signals = detector.detect();
		assert!(signals.iter().any(|s| matches!(s, DriftSignal::TunnelVision { .. })));
	}

	#[test]
	fn detect_risk_aversion() {
		let config = DriftDetectorConfig::default().with_risk_aversion_threshold(50.0);
		let mut detector = DriftDetector::new(config);

		// Add small changes
		for _ in 0..10 {
			detector.record_action(AgentAction::new("edit").with_change_size(5.0));
		}

		let signals = detector.detect();
		assert!(signals.iter().any(|s| matches!(s, DriftSignal::RiskAversion { .. })));
	}

	#[test]
	fn detect_circular_reasoning() {
		let mut detector = DriftDetector::with_defaults();

		// Add repeated similar actions
		for _ in 0..6 {
			detector.record_action(AgentAction::new("edit").with_topics(vec!["same_file"]));
		}

		let signals = detector.detect();
		assert!(signals.iter().any(|s| matches!(s, DriftSignal::CircularReasoning { .. })));
	}

	#[test]
	fn no_false_positives() {
		let mut detector = DriftDetector::with_defaults();

		// Add varied, successful actions
		detector.record_verification(make_verification(
			true,
			ClaimType::Compiles { files: vec![] },
		));
		detector.record_verification(make_verification(
			true,
			ClaimType::TestsPass {
				test_pattern: "test_*".to_string(),
			},
		));

		detector.record_action(AgentAction::new("edit").with_topics(vec!["src"]).with_change_size(50.0));
		detector.record_action(AgentAction::new("test").with_topics(vec!["tests"]).with_change_size(30.0));
		detector.record_action(AgentAction::new("docs").with_topics(vec!["docs"]).with_change_size(20.0));

		let signals = detector.detect();
		// Only staleness might trigger (if test runs slow), filter it out
		let non_stale: Vec<_> = signals
			.into_iter()
			.filter(|s| !matches!(s, DriftSignal::StaleContext { .. }))
			.collect();
		assert!(non_stale.is_empty());
	}
}
