// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Configuration types for the multi-agent system.

use serde::{Deserialize, Serialize};

use crate::error::MultiAgentError;

/// Configuration for a multi-agent project run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ProjectConfig {
	/// Maximum number of concurrent workers
	pub max_workers: u32,
	/// Maximum number of concurrent planners
	pub max_planners: u32,
	/// Model configurations by role
	pub model_configs: Vec<RoleModelConfig>,
	/// Evaluation criteria for judge
	pub evaluation_criteria: EvaluationCriteria,
	/// Whether to auto-commit changes
	pub auto_commit: bool,
	/// Maximum iterations per task
	pub max_iterations: u32,
	/// Checkpoint interval in seconds (0 = disabled)
	pub checkpoint_interval_secs: u64,
}

impl Default for ProjectConfig {
	fn default() -> Self {
		Self {
			max_workers: 10,
			max_planners: 5,
			model_configs: Vec::new(),
			evaluation_criteria: EvaluationCriteria::default(),
			auto_commit: true,
			max_iterations: 3,
			checkpoint_interval_secs: 300,
		}
	}
}

/// Model configuration for a specific role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct RoleModelConfig {
	/// Agent role
	pub role: AgentRole,
	/// Model configuration
	pub model: ModelConfig,
}

/// Configuration for an LLM model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ModelConfig {
	/// Model identifier (e.g., "claude-opus-4-5-20251101")
	pub model_id: String,
	/// Provider (e.g., "anthropic", "openai")
	pub provider: String,
	/// Temperature (0.0 to 1.0)
	pub temperature: f32,
	/// Maximum tokens
	pub max_tokens: u32,
	/// System prompt suffix
	pub system_prompt_suffix: Option<String>,
}

impl Default for ModelConfig {
	fn default() -> Self {
		Self {
			model_id: "claude-sonnet-4-20250514".to_string(),
			provider: "anthropic".to_string(),
			temperature: 0.5,
			max_tokens: 8192,
			system_prompt_suffix: None,
		}
	}
}

/// Role of an agent in the multi-agent system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
	/// High-level planning and coordination
	Planner,
	/// Code implementation
	Worker,
	/// Quality evaluation
	Judge,
	/// Code exploration and analysis
	Explorer,
}

impl std::fmt::Display for AgentRole {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AgentRole::Planner => write!(f, "planner"),
			AgentRole::Worker => write!(f, "worker"),
			AgentRole::Judge => write!(f, "judge"),
			AgentRole::Explorer => write!(f, "explorer"),
		}
	}
}

impl std::str::FromStr for AgentRole {
	type Err = MultiAgentError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"planner" => Ok(AgentRole::Planner),
			"worker" => Ok(AgentRole::Worker),
			"judge" => Ok(AgentRole::Judge),
			"explorer" => Ok(AgentRole::Explorer),
			_ => Err(MultiAgentError::InvalidAgentRole(s.to_string())),
		}
	}
}

/// Criteria for evaluating task completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct EvaluationCriteria {
	/// Required test pass rate (0.0 to 1.0)
	pub min_test_pass_rate: f64,
	/// Required code quality score (0.0 to 1.0)
	pub min_quality_score: f64,
	/// Maximum iterations before escalation
	pub max_iterations: u32,
	/// Whether compilation must succeed
	pub require_compilation: bool,
	/// Whether linting must pass
	pub require_lint_pass: bool,
}

impl Default for EvaluationCriteria {
	fn default() -> Self {
		Self {
			min_test_pass_rate: 1.0,
			min_quality_score: 0.8,
			max_iterations: 3,
			require_compilation: true,
			require_lint_pass: true,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_project_config_default() {
		let config = ProjectConfig::default();
		assert_eq!(config.max_workers, 10);
		assert_eq!(config.max_planners, 5);
		assert!(config.auto_commit);
	}

	#[test]
	fn test_model_config_default() {
		let config = ModelConfig::default();
		assert_eq!(config.provider, "anthropic");
		assert!(config.temperature > 0.0);
	}

	#[test]
	fn test_agent_role_display() {
		assert_eq!(AgentRole::Planner.to_string(), "planner");
		assert_eq!(AgentRole::Worker.to_string(), "worker");
		assert_eq!(AgentRole::Judge.to_string(), "judge");
		assert_eq!(AgentRole::Explorer.to_string(), "explorer");
	}

	#[test]
	fn test_agent_role_parse() {
		assert_eq!("planner".parse::<AgentRole>().unwrap(), AgentRole::Planner);
		assert_eq!("worker".parse::<AgentRole>().unwrap(), AgentRole::Worker);
		assert!("invalid".parse::<AgentRole>().is_err());
	}

	#[test]
	fn test_evaluation_criteria_default() {
		let criteria = EvaluationCriteria::default();
		assert_eq!(criteria.min_test_pass_rate, 1.0);
		assert!(criteria.require_compilation);
	}
}
