// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Domain types for the multi-agent system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{DomainId, PlannerId, WorkerId};

/// A domain (area) of the codebase managed by a planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Domain {
	/// Domain identifier
	pub id: DomainId,
	/// Human-readable name
	pub name: String,
	/// File patterns this domain covers (glob patterns)
	pub file_patterns: Vec<String>,
	/// Dependencies on other domains
	pub dependencies: Vec<DomainId>,
	/// Created timestamp
	pub created_at: DateTime<Utc>,
}

impl Domain {
	/// Create a new domain.
	#[must_use]
	pub fn new(name: String, file_patterns: Vec<String>) -> Self {
		Self {
			id: DomainId::new(),
			name,
			file_patterns,
			dependencies: Vec::new(),
			created_at: Utc::now(),
		}
	}
}

/// State of a domain during a project run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DomainState {
	/// Domain ID
	pub domain_id: DomainId,
	/// Active planner for this domain
	pub planner_id: Option<PlannerId>,
	/// Active workers in this domain
	pub active_workers: Vec<WorkerInfo>,
	/// Tasks pending in this domain
	pub pending_tasks: u32,
	/// Tasks completed in this domain
	pub completed_tasks: u32,
	/// Tasks failed in this domain
	pub failed_tasks: u32,
	/// Domain health status
	pub health: DomainHealth,
}

/// Information about an active worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WorkerInfo {
	/// Worker ID
	pub worker_id: WorkerId,
	/// Current task ID (if any)
	pub current_task_id: Option<crate::ids::TaskId>,
	/// When the worker started on current task
	pub started_at: Option<DateTime<Utc>>,
}

/// Health status of a domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum DomainHealth {
	/// Domain is healthy
	#[default]
	Healthy,
	/// Domain has warnings
	Warning,
	/// Domain has errors
	Error,
	/// Domain is stalled
	Stalled,
}

impl std::fmt::Display for DomainHealth {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			DomainHealth::Healthy => write!(f, "healthy"),
			DomainHealth::Warning => write!(f, "warning"),
			DomainHealth::Error => write!(f, "error"),
			DomainHealth::Stalled => write!(f, "stalled"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_domain_new() {
		let domain = Domain::new(
			"frontend".to_string(),
			vec!["web/**/*.ts".to_string(), "web/**/*.tsx".to_string()],
		);
		assert_eq!(domain.name, "frontend");
		assert_eq!(domain.file_patterns.len(), 2);
		assert!(domain.dependencies.is_empty());
	}

	#[test]
	fn test_domain_health_display() {
		assert_eq!(DomainHealth::Healthy.to_string(), "healthy");
		assert_eq!(DomainHealth::Stalled.to_string(), "stalled");
	}

	#[test]
	fn test_domain_state_default() {
		let state = DomainState::default();
		assert!(state.planner_id.is_none());
		assert!(state.active_workers.is_empty());
		assert_eq!(state.health, DomainHealth::Healthy);
	}
}
