// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Judge agent - evaluates task completions.

use tracing::instrument;

use loom_multi_agent_core::{EvaluationCriteria, TaskCompletion, Verdict};

use crate::Result;

/// Judge agent that evaluates task completions.
pub struct JudgeAgent {
	/// Evaluation criteria
	criteria: EvaluationCriteria,
}

impl JudgeAgent {
	/// Create a new judge agent.
	pub fn new(criteria: EvaluationCriteria) -> Self {
		Self { criteria }
	}

	/// Create a judge with default criteria.
	pub fn with_defaults() -> Self {
		Self::new(EvaluationCriteria::default())
	}

	/// Evaluate a task completion.
	#[instrument(skip(self, completion), fields(task_id = %completion.task_id))]
	pub async fn evaluate(&self, completion: &TaskCompletion) -> Result<Verdict> {
		let all_passed = completion
			.verifications
			.iter()
			.all(|v| v.passed);

		if !all_passed {
			let failures: Vec<_> = completion
				.verifications
				.iter()
				.filter(|v| !v.passed)
				.map(|v| v.criterion.clone())
				.collect();

			return Ok(Verdict::needs_iteration(
				format!("Verification failures: {}", failures.join(", ")),
				failures,
			));
		}

		if completion.output.summary.is_empty() {
			return Ok(Verdict::needs_iteration(
				"Output summary is empty".to_string(),
				vec!["Provide a meaningful summary".to_string()],
			));
		}

		Ok(Verdict::approved(format!(
			"Task completed successfully: {}",
			completion.output.summary
		)))
	}

	/// Determine if a task needs iteration.
	#[instrument(skip(self, completion), fields(task_id = %completion.task_id))]
	pub async fn needs_iteration(&self, completion: &TaskCompletion) -> Result<bool> {
		let verdict = self.evaluate(completion).await?;
		Ok(verdict.needs_work())
	}

	/// Check if maximum iterations have been reached.
	pub fn max_iterations_reached(&self, current_iteration: u32) -> bool {
		current_iteration >= self.criteria.max_iterations
	}

	/// Get the evaluation criteria.
	pub fn criteria(&self) -> &EvaluationCriteria {
		&self.criteria
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_multi_agent_core::{TaskId, TaskOutput, WorkerId, VerificationResult};

	#[tokio::test]
	async fn test_judge_approve_success() {
		let judge = JudgeAgent::with_defaults();

		let completion = TaskCompletion {
			task_id: TaskId::new(),
			worker_id: WorkerId::new(),
			completed_at: chrono::Utc::now(),
			duration_ms: 100,
			output: TaskOutput {
				summary: "Task completed".to_string(),
				notes: vec![],
				artifacts: vec![],
				suggested_tasks: vec![],
				learnings: vec![],
			},
			changes: vec![],
			commit_sha: None,
			verifications: vec![VerificationResult {
				criterion: "test".to_string(),
				passed: true,
				output: None,
				error: None,
			}],
			context_snapshot: Default::default(),
			unblocked_tasks: Vec::new(),
		};

		let verdict = judge.evaluate(&completion).await.unwrap();
		assert!(verdict.is_approved());
	}

	#[tokio::test]
	async fn test_judge_needs_iteration_on_failure() {
		let judge = JudgeAgent::with_defaults();

		let completion = TaskCompletion {
			task_id: TaskId::new(),
			worker_id: WorkerId::new(),
			completed_at: chrono::Utc::now(),
			duration_ms: 100,
			output: TaskOutput {
				summary: "Task completed".to_string(),
				notes: vec![],
				artifacts: vec![],
				suggested_tasks: vec![],
				learnings: vec![],
			},
			changes: vec![],
			commit_sha: None,
			verifications: vec![VerificationResult {
				criterion: "test".to_string(),
				passed: false,
				output: None,
				error: Some("Test failed".to_string()),
			}],
			context_snapshot: Default::default(),
			unblocked_tasks: Vec::new(),
		};

		let verdict = judge.evaluate(&completion).await.unwrap();
		assert!(verdict.needs_work());
	}

	#[test]
	fn test_max_iterations() {
		let judge = JudgeAgent::with_defaults();
		assert!(!judge.max_iterations_reached(0));
		assert!(!judge.max_iterations_reached(2));
		assert!(judge.max_iterations_reached(3));
		assert!(judge.max_iterations_reached(5));
	}
}
