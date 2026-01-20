// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Verdict types for judge evaluation.

use serde::{Deserialize, Serialize};

/// Result of judge evaluation of a task completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
	/// Task completed successfully
	Approved { feedback: String },
	/// Task needs more work
	NeedsIteration {
		feedback: String,
		suggestions: Vec<String>,
	},
	/// Task cannot be completed, escalate to planner
	Escalate { reason: String },
}

impl Verdict {
	/// Create an approved verdict.
	#[must_use]
	pub fn approved(feedback: impl Into<String>) -> Self {
		Verdict::Approved {
			feedback: feedback.into(),
		}
	}

	/// Create a needs iteration verdict.
	#[must_use]
	pub fn needs_iteration(feedback: impl Into<String>, suggestions: Vec<String>) -> Self {
		Verdict::NeedsIteration {
			feedback: feedback.into(),
			suggestions,
		}
	}

	/// Create an escalate verdict.
	#[must_use]
	pub fn escalate(reason: impl Into<String>) -> Self {
		Verdict::Escalate {
			reason: reason.into(),
		}
	}

	/// Check if the verdict is approved.
	#[must_use]
	pub fn is_approved(&self) -> bool {
		matches!(self, Verdict::Approved { .. })
	}

	/// Check if the verdict requires iteration.
	#[must_use]
	pub fn needs_work(&self) -> bool {
		matches!(self, Verdict::NeedsIteration { .. })
	}

	/// Check if the verdict requires escalation.
	#[must_use]
	pub fn should_escalate(&self) -> bool {
		matches!(self, Verdict::Escalate { .. })
	}
}

impl std::fmt::Display for Verdict {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Verdict::Approved { .. } => write!(f, "approved"),
			Verdict::NeedsIteration { .. } => write!(f, "needs_iteration"),
			Verdict::Escalate { .. } => write!(f, "escalate"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_verdict_approved() {
		let verdict = Verdict::approved("Well done");
		assert!(verdict.is_approved());
		assert!(!verdict.needs_work());
		assert!(!verdict.should_escalate());
	}

	#[test]
	fn test_verdict_needs_iteration() {
		let verdict = Verdict::needs_iteration("Missing tests", vec!["Add unit tests".to_string()]);
		assert!(!verdict.is_approved());
		assert!(verdict.needs_work());
		assert!(!verdict.should_escalate());
	}

	#[test]
	fn test_verdict_escalate() {
		let verdict = Verdict::escalate("Requires architectural decision");
		assert!(!verdict.is_approved());
		assert!(!verdict.needs_work());
		assert!(verdict.should_escalate());
	}

	#[test]
	fn test_verdict_display() {
		assert_eq!(Verdict::approved("ok").to_string(), "approved");
		assert_eq!(
			Verdict::needs_iteration("fix", vec![]).to_string(),
			"needs_iteration"
		);
		assert_eq!(Verdict::escalate("help").to_string(), "escalate");
	}
}
