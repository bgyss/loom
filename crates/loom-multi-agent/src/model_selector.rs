// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Model selector - role-based model selection.

use std::collections::HashMap;

use loom_multi_agent_core::{AgentRole, ModelConfig};

/// Selects appropriate models based on agent roles.
pub struct ModelSelector {
	/// Model configurations by role
	role_models: HashMap<AgentRole, ModelConfig>,
	/// Default model
	default_model: ModelConfig,
}

impl ModelSelector {
	/// Create a new model selector with custom configurations.
	pub fn new(role_models: HashMap<AgentRole, ModelConfig>, default_model: ModelConfig) -> Self {
		Self {
			role_models,
			default_model,
		}
	}

	/// Create a model selector with recommended configurations.
	pub fn recommended() -> Self {
		let mut role_models = HashMap::new();

		role_models.insert(
			AgentRole::Planner,
			ModelConfig {
				model_id: "claude-opus-4-5-20251101".to_string(),
				provider: "anthropic".to_string(),
				temperature: 0.7,
				max_tokens: 8192,
				system_prompt_suffix: Some(
					"Focus on strategic planning and task decomposition.".to_string(),
				),
			},
		);

		role_models.insert(
			AgentRole::Worker,
			ModelConfig {
				model_id: "claude-sonnet-4-20250514".to_string(),
				provider: "anthropic".to_string(),
				temperature: 0.3,
				max_tokens: 16384,
				system_prompt_suffix: Some(
					"Focus on precise implementation and testing.".to_string(),
				),
			},
		);

		role_models.insert(
			AgentRole::Judge,
			ModelConfig {
				model_id: "claude-sonnet-4-20250514".to_string(),
				provider: "anthropic".to_string(),
				temperature: 0.2,
				max_tokens: 4096,
				system_prompt_suffix: Some("Evaluate objectively against criteria.".to_string()),
			},
		);

		role_models.insert(
			AgentRole::Explorer,
			ModelConfig {
				model_id: "claude-sonnet-4-20250514".to_string(),
				provider: "anthropic".to_string(),
				temperature: 0.5,
				max_tokens: 8192,
				system_prompt_suffix: Some(
					"Focus on thorough exploration and analysis.".to_string(),
				),
			},
		);

		let default_model = ModelConfig {
			model_id: "claude-sonnet-4-20250514".to_string(),
			provider: "anthropic".to_string(),
			temperature: 0.5,
			max_tokens: 8192,
			system_prompt_suffix: None,
		};

		Self {
			role_models,
			default_model,
		}
	}

	/// Get the model configuration for a specific role.
	pub fn for_role(&self, role: AgentRole) -> &ModelConfig {
		self.role_models.get(&role).unwrap_or(&self.default_model)
	}

	/// Get the default model.
	pub fn default_model(&self) -> &ModelConfig {
		&self.default_model
	}

	/// Check if a role has a custom model.
	pub fn has_custom_model(&self, role: AgentRole) -> bool {
		self.role_models.contains_key(&role)
	}

	/// Set the model for a specific role.
	pub fn set_model(&mut self, role: AgentRole, config: ModelConfig) {
		self.role_models.insert(role, config);
	}
}

impl Default for ModelSelector {
	fn default() -> Self {
		Self::recommended()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_model_selector_recommended() {
		let selector = ModelSelector::recommended();

		let planner_model = selector.for_role(AgentRole::Planner);
		assert!(planner_model.model_id.contains("opus"));

		let worker_model = selector.for_role(AgentRole::Worker);
		assert!(worker_model.model_id.contains("sonnet"));
	}

	#[test]
	fn test_model_selector_default() {
		let selector = ModelSelector::recommended();
		let default = selector.default_model();
		assert!(default.model_id.contains("sonnet"));
	}

	#[test]
	fn test_has_custom_model() {
		let selector = ModelSelector::recommended();
		assert!(selector.has_custom_model(AgentRole::Planner));
		assert!(selector.has_custom_model(AgentRole::Worker));
	}

	#[test]
	fn test_set_model() {
		let mut selector = ModelSelector::recommended();

		let custom = ModelConfig {
			model_id: "custom-model".to_string(),
			provider: "custom".to_string(),
			temperature: 0.1,
			max_tokens: 1000,
			system_prompt_suffix: None,
		};

		selector.set_model(AgentRole::Explorer, custom);

		let explorer_model = selector.for_role(AgentRole::Explorer);
		assert_eq!(explorer_model.model_id, "custom-model");
	}
}
