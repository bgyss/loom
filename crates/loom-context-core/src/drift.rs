// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Drift detection types for context management.
//!
//! Drift signals indicate when an agent's context or reasoning may be degrading,
//! including high failure rates, repeated errors, circular reasoning, and tunnel vision.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::ids::{DriftSignalId, OrgId};

/// Types of drift signals that indicate context degradation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DriftSignal {
	/// High verification failure rate.
	HighFailureRate { rate: f64 },
	/// Repeated failures on same claim type.
	RepeatedFailures { claim_type: String, count: u32 },
	/// Circular reasoning detected.
	CircularReasoning { pattern: String },
	/// Contradictory claims detected.
	Contradiction { claim_a: String, claim_b: String },
	/// Tunnel vision (narrow focus on single topic).
	TunnelVision {
		topic_distribution: HashMap<String, f64>,
	},
	/// Risk aversion (only small changes).
	RiskAversion { avg_change_size: f64, threshold: f64 },
	/// Context staleness (too old).
	StaleContext { age: Duration },
}

impl DriftSignal {
	/// Returns the signal type as a string.
	pub fn signal_type(&self) -> &'static str {
		match self {
			Self::HighFailureRate { .. } => "high_failure_rate",
			Self::RepeatedFailures { .. } => "repeated_failures",
			Self::CircularReasoning { .. } => "circular_reasoning",
			Self::Contradiction { .. } => "contradiction",
			Self::TunnelVision { .. } => "tunnel_vision",
			Self::RiskAversion { .. } => "risk_aversion",
			Self::StaleContext { .. } => "stale_context",
		}
	}

	/// Returns a human-readable description of the drift signal.
	pub fn description(&self) -> String {
		match self {
			Self::HighFailureRate { rate } => {
				format!("High verification failure rate: {:.1}%", rate * 100.0)
			}
			Self::RepeatedFailures { claim_type, count } => {
				format!(
					"Repeated failures on claim type '{}': {} failures",
					claim_type, count
				)
			}
			Self::CircularReasoning { pattern } => {
				format!("Circular reasoning detected: {}", pattern)
			}
			Self::Contradiction { claim_a, claim_b } => {
				format!(
					"Contradictory claims: '{}' vs '{}'",
					claim_a.chars().take(50).collect::<String>(),
					claim_b.chars().take(50).collect::<String>()
				)
			}
			Self::TunnelVision { topic_distribution } => {
				let dominant = topic_distribution
					.iter()
					.max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
					.map(|(k, v)| format!("{} at {:.1}%", k, v * 100.0))
					.unwrap_or_default();
				format!("Tunnel vision: focus on {}", dominant)
			}
			Self::RiskAversion {
				avg_change_size,
				threshold,
			} => {
				format!(
					"Risk aversion: avg change size {:.1} below threshold {:.1}",
					avg_change_size, threshold
				)
			}
			Self::StaleContext { age } => {
				format!("Context staleness: {:.1} minutes old", age.as_secs_f64() / 60.0)
			}
		}
	}

	/// Returns the severity level (1-5, 5 being most severe).
	pub fn severity(&self) -> u8 {
		match self {
			Self::HighFailureRate { rate } => {
				if *rate > 0.8 {
					5
				} else if *rate > 0.6 {
					4
				} else {
					3
				}
			}
			Self::RepeatedFailures { count, .. } => {
				if *count > 10 {
					5
				} else if *count > 5 {
					4
				} else {
					3
				}
			}
			Self::CircularReasoning { .. } => 4,
			Self::Contradiction { .. } => 5,
			Self::TunnelVision { .. } => 3,
			Self::RiskAversion { .. } => 2,
			Self::StaleContext { age } => {
				if age.as_secs() > 3600 {
					4
				} else {
					2
				}
			}
		}
	}
}

/// A persisted drift signal record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftSignalRecord {
	/// Unique identifier.
	pub id: DriftSignalId,
	/// The drift signal.
	pub signal: DriftSignal,
	/// When detected.
	pub detected_at: DateTime<Utc>,
	/// When corrected (if corrected).
	pub corrected_at: Option<DateTime<Utc>>,
	/// Correction action taken.
	pub correction_action: Option<CorrectionAction>,
	/// Organization ID for multi-tenancy.
	pub org_id: OrgId,
}

impl DriftSignalRecord {
	/// Creates a new drift signal record.
	pub fn new(org_id: OrgId, signal: DriftSignal) -> Self {
		Self {
			id: DriftSignalId::new(),
			signal,
			detected_at: Utc::now(),
			corrected_at: None,
			correction_action: None,
			org_id,
		}
	}

	/// Marks the signal as corrected with the given action.
	pub fn mark_corrected(&mut self, action: CorrectionAction) {
		self.corrected_at = Some(Utc::now());
		self.correction_action = Some(action);
	}

	/// Returns true if this signal has been corrected.
	pub fn is_corrected(&self) -> bool {
		self.corrected_at.is_some()
	}
}

/// Actions taken to correct drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CorrectionAction {
	/// Refreshed context from source.
	RefreshedContext,
	/// Forced exploration of neglected areas.
	ForcedExploration { areas: Vec<String> },
	/// Injected prompt to change behavior.
	PromptInjection { prompt: String },
	/// Generated a fresh approach.
	FreshApproach { approach: String },
	/// Resolved a contradiction.
	ResolvedContradiction { resolution: String },
	/// Escalated to human operator.
	Escalated { reason: String },
}

impl CorrectionAction {
	/// Returns the action type as a string.
	pub fn action_type(&self) -> &'static str {
		match self {
			Self::RefreshedContext => "refreshed_context",
			Self::ForcedExploration { .. } => "forced_exploration",
			Self::PromptInjection { .. } => "prompt_injection",
			Self::FreshApproach { .. } => "fresh_approach",
			Self::ResolvedContradiction { .. } => "resolved_contradiction",
			Self::Escalated { .. } => "escalated",
		}
	}
}

/// Configuration for drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftDetectorConfig {
	/// Failure rate threshold for triggering high failure rate signal.
	pub max_failure_rate: f64,
	/// Number of actions to track for pattern detection.
	pub action_history_size: usize,
	/// Tunnel vision threshold (max % on single topic).
	pub tunnel_vision_threshold: f64,
	/// Risk aversion change size threshold.
	pub risk_aversion_threshold: f64,
	/// Context staleness threshold.
	pub staleness_threshold: Duration,
	/// Repeated failures count threshold.
	pub repeated_failures_threshold: u32,
}

impl Default for DriftDetectorConfig {
	fn default() -> Self {
		Self {
			max_failure_rate: 0.5,
			action_history_size: 50,
			tunnel_vision_threshold: 0.7,
			risk_aversion_threshold: 10.0,
			staleness_threshold: Duration::from_secs(3600), // 1 hour
			repeated_failures_threshold: 3,
		}
	}
}

impl DriftDetectorConfig {
	/// Creates a new config with default values.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the maximum failure rate.
	pub fn with_max_failure_rate(mut self, rate: f64) -> Self {
		self.max_failure_rate = rate;
		self
	}

	/// Sets the action history size.
	pub fn with_action_history_size(mut self, size: usize) -> Self {
		self.action_history_size = size;
		self
	}

	/// Sets the tunnel vision threshold.
	pub fn with_tunnel_vision_threshold(mut self, threshold: f64) -> Self {
		self.tunnel_vision_threshold = threshold;
		self
	}

	/// Sets the risk aversion threshold.
	pub fn with_risk_aversion_threshold(mut self, threshold: f64) -> Self {
		self.risk_aversion_threshold = threshold;
		self
	}

	/// Sets the staleness threshold.
	pub fn with_staleness_threshold(mut self, threshold: Duration) -> Self {
		self.staleness_threshold = threshold;
		self
	}

	/// Sets the repeated failures threshold.
	pub fn with_repeated_failures_threshold(mut self, threshold: u32) -> Self {
		self.repeated_failures_threshold = threshold;
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn drift_signal_types() {
		let signal = DriftSignal::HighFailureRate { rate: 0.75 };
		assert_eq!(signal.signal_type(), "high_failure_rate");
		assert!(!signal.description().is_empty());
		assert!(signal.severity() >= 3);
	}

	#[test]
	fn drift_signal_severity_scaling() {
		// High failure rate severity increases with rate
		let low = DriftSignal::HighFailureRate { rate: 0.5 };
		let mid = DriftSignal::HighFailureRate { rate: 0.7 };
		let high = DriftSignal::HighFailureRate { rate: 0.9 };

		assert!(low.severity() <= mid.severity());
		assert!(mid.severity() <= high.severity());
	}

	#[test]
	fn drift_signal_record_correction() {
		let org_id = OrgId::new();
		let signal = DriftSignal::TunnelVision {
			topic_distribution: HashMap::from([("tests".to_string(), 0.8)]),
		};
		let mut record = DriftSignalRecord::new(org_id, signal);

		assert!(!record.is_corrected());

		record.mark_corrected(CorrectionAction::ForcedExploration {
			areas: vec!["documentation".to_string()],
		});

		assert!(record.is_corrected());
		assert!(record.corrected_at.is_some());
		assert!(record.correction_action.is_some());
	}

	#[test]
	fn correction_action_types() {
		let actions = vec![
			CorrectionAction::RefreshedContext,
			CorrectionAction::ForcedExploration {
				areas: vec!["test".to_string()],
			},
			CorrectionAction::PromptInjection {
				prompt: "test".to_string(),
			},
			CorrectionAction::FreshApproach {
				approach: "test".to_string(),
			},
			CorrectionAction::ResolvedContradiction {
				resolution: "test".to_string(),
			},
			CorrectionAction::Escalated {
				reason: "test".to_string(),
			},
		];

		for action in actions {
			assert!(!action.action_type().is_empty());
		}
	}

	#[test]
	fn default_drift_config() {
		let config = DriftDetectorConfig::default();
		assert!((config.max_failure_rate - 0.5).abs() < f64::EPSILON);
		assert_eq!(config.action_history_size, 50);
		assert!((config.tunnel_vision_threshold - 0.7).abs() < f64::EPSILON);
	}
}
