// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Verification types for grounding reasoning in executable reality.
//!
//! The verification system provides claim types and results for verifying
//! agent assertions through executable checks like compilation, tests, and commands.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::ids::{OrgId, VerificationId};

/// A claim that can be verified through execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
	/// What is being claimed.
	pub assertion: String,
	/// Type of claim.
	pub claim_type: ClaimType,
	/// Evidence/context supporting the claim.
	pub evidence: Vec<String>,
}

impl Claim {
	/// Creates a new claim.
	pub fn new(assertion: impl Into<String>, claim_type: ClaimType) -> Self {
		Self {
			assertion: assertion.into(),
			claim_type,
			evidence: Vec::new(),
		}
	}

	/// Adds evidence to the claim.
	pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
		self.evidence.push(evidence.into());
		self
	}

	/// Adds multiple pieces of evidence to the claim.
	pub fn with_evidences(mut self, evidence: impl IntoIterator<Item = impl Into<String>>) -> Self {
		self.evidence.extend(evidence.into_iter().map(Into::into));
		self
	}
}

/// Types of claims that can be verified.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaimType {
	/// Code compiles successfully.
	Compiles { files: Vec<PathBuf> },
	/// Tests pass.
	TestsPass { test_pattern: String },
	/// File contains a pattern.
	FileContains { path: PathBuf, pattern: String },
	/// API returns expected result.
	ApiReturns { endpoint: String, expected_status: u16 },
	/// Command succeeds.
	CommandSucceeds { command: String },
	/// Type checks pass.
	TypeChecks { files: Vec<PathBuf> },
	/// Lint checks pass.
	LintPasses { files: Vec<PathBuf> },
}

impl ClaimType {
	/// Returns the claim type as a string key.
	pub fn type_key(&self) -> &'static str {
		match self {
			Self::Compiles { .. } => "compiles",
			Self::TestsPass { .. } => "tests_pass",
			Self::FileContains { .. } => "file_contains",
			Self::ApiReturns { .. } => "api_returns",
			Self::CommandSucceeds { .. } => "command_succeeds",
			Self::TypeChecks { .. } => "type_checks",
			Self::LintPasses { .. } => "lint_passes",
		}
	}
}

/// Result of verifying a claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
	/// Unique identifier.
	pub id: VerificationId,
	/// Claim that was verified.
	pub claim: Claim,
	/// Whether verification passed.
	pub passed: bool,
	/// Output/evidence from verification.
	pub output: String,
	/// Timestamp of verification.
	pub verified_at: DateTime<Utc>,
	/// Duration of verification.
	pub duration: Duration,
	/// Associated task ID (if any).
	pub task_id: Option<String>,
	/// Organization ID for multi-tenancy.
	pub org_id: OrgId,
}

impl VerificationResult {
	/// Creates a new successful verification result.
	pub fn success(org_id: OrgId, claim: Claim, output: impl Into<String>, duration: Duration) -> Self {
		Self {
			id: VerificationId::new(),
			claim,
			passed: true,
			output: output.into(),
			verified_at: Utc::now(),
			duration,
			task_id: None,
			org_id,
		}
	}

	/// Creates a new failed verification result.
	pub fn failure(org_id: OrgId, claim: Claim, output: impl Into<String>, duration: Duration) -> Self {
		Self {
			id: VerificationId::new(),
			claim,
			passed: false,
			output: output.into(),
			verified_at: Utc::now(),
			duration,
			task_id: None,
			org_id,
		}
	}

	/// Associates a task ID with this result.
	pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
		self.task_id = Some(task_id.into());
		self
	}
}

/// Summary of feedback from verifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedbackSummary {
	/// Number of verifications that passed.
	pub passed: u32,
	/// Number of verifications that failed.
	pub failed: u32,
	/// Number of learnings added to memory.
	pub learnings_added: u32,
	/// Number of errors recorded.
	pub errors_recorded: u32,
}

impl FeedbackSummary {
	/// Returns the total number of verifications.
	pub fn total(&self) -> u32 {
		self.passed + self.failed
	}

	/// Returns the success rate (0.0 - 1.0).
	pub fn success_rate(&self) -> f64 {
		let total = self.total();
		if total == 0 {
			1.0
		} else {
			self.passed as f64 / total as f64
		}
	}

	/// Returns the failure rate (0.0 - 1.0).
	pub fn failure_rate(&self) -> f64 {
		1.0 - self.success_rate()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	#[test]
	fn claim_creation() {
		let claim = Claim::new(
			"code compiles",
			ClaimType::Compiles {
				files: vec![PathBuf::from("src/main.rs")],
			},
		)
		.with_evidence("checked with cargo build");

		assert_eq!(claim.assertion, "code compiles");
		assert_eq!(claim.evidence.len(), 1);
		assert_eq!(claim.claim_type.type_key(), "compiles");
	}

	#[test]
	fn verification_result_success() {
		let org_id = OrgId::new();
		let claim = Claim::new(
			"tests pass",
			ClaimType::TestsPass {
				test_pattern: "test_*".to_string(),
			},
		);
		let result =
			VerificationResult::success(org_id, claim, "10 tests passed", Duration::from_secs(5));

		assert!(result.passed);
		assert_eq!(result.output, "10 tests passed");
	}

	#[test]
	fn verification_result_failure() {
		let org_id = OrgId::new();
		let claim = Claim::new(
			"command succeeds",
			ClaimType::CommandSucceeds {
				command: "exit 1".to_string(),
			},
		);
		let result = VerificationResult::failure(org_id, claim, "exit code 1", Duration::from_millis(100));

		assert!(!result.passed);
	}

	#[test]
	fn feedback_summary_rates() {
		let mut summary = FeedbackSummary::default();
		summary.passed = 8;
		summary.failed = 2;

		assert_eq!(summary.total(), 10);
		assert!((summary.success_rate() - 0.8).abs() < f64::EPSILON);
		assert!((summary.failure_rate() - 0.2).abs() < f64::EPSILON);
	}

	#[test]
	fn feedback_summary_empty() {
		let summary = FeedbackSummary::default();
		assert_eq!(summary.total(), 0);
		// Empty should have 100% success rate (no failures)
		assert!((summary.success_rate() - 1.0).abs() < f64::EPSILON);
	}

	proptest! {
		#[test]
		fn feedback_summary_rates_bounded(passed in 0u32..1000, failed in 0u32..1000) {
			let mut summary = FeedbackSummary::default();
			summary.passed = passed;
			summary.failed = failed;

			let rate = summary.success_rate();
			prop_assert!(rate >= 0.0);
			prop_assert!(rate <= 1.0);

			let failure_rate = summary.failure_rate();
			prop_assert!(failure_rate >= 0.0);
			prop_assert!(failure_rate <= 1.0);

			prop_assert!((rate + failure_rate - 1.0).abs() < f64::EPSILON);
		}
	}
}
