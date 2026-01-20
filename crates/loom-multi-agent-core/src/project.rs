// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Project state types for the multi-agent system.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::DomainState;
use crate::ids::{CheckpointId, DomainId, ProjectRunId};

/// Global state of a multi-agent project run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ProjectState {
	/// Project run identifier
	pub id: ProjectRunId,
	/// Organization ID
	pub org_id: String,
	/// Path to the project
	pub project_path: String,
	/// Overall goal
	pub goal: String,
	/// Current phase
	pub phase: ProjectPhase,
	/// Project status
	pub status: ProjectStatus,
	/// Domain states
	pub domains: HashMap<DomainId, DomainState>,
	/// Global metrics
	pub metrics: ProjectMetrics,
	/// Active checkpoint IDs
	pub checkpoints: Vec<CheckpointId>,
	/// Started timestamp
	pub started_at: DateTime<Utc>,
	/// Completed timestamp
	pub completed_at: Option<DateTime<Utc>>,
	/// User who created this run
	pub created_by: String,
}

impl ProjectState {
	/// Create a new project state.
	#[must_use]
	pub fn new(org_id: String, project_path: String, goal: String, created_by: String) -> Self {
		Self {
			id: ProjectRunId::new(),
			org_id,
			project_path,
			goal,
			phase: ProjectPhase::Exploration,
			status: ProjectStatus::Running,
			domains: HashMap::new(),
			metrics: ProjectMetrics::default(),
			checkpoints: Vec::new(),
			started_at: Utc::now(),
			completed_at: None,
			created_by,
		}
	}
}

/// Phase of a project run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum ProjectPhase {
	/// Initial exploration
	Exploration,
	/// Active development
	Development { iteration: u32 },
	/// Refinement and polish
	Refinement,
	/// Final verification
	Verification,
	/// Complete
	Complete,
}

impl std::fmt::Display for ProjectPhase {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ProjectPhase::Exploration => write!(f, "exploration"),
			ProjectPhase::Development { iteration } => write!(f, "development:{}", iteration),
			ProjectPhase::Refinement => write!(f, "refinement"),
			ProjectPhase::Verification => write!(f, "verification"),
			ProjectPhase::Complete => write!(f, "complete"),
		}
	}
}

impl std::str::FromStr for ProjectPhase {
	type Err = crate::error::MultiAgentError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.starts_with("development:") {
			let iter_str = s.strip_prefix("development:").unwrap_or("0");
			let iteration = iter_str
				.parse()
				.map_err(|_| crate::error::MultiAgentError::InvalidProjectPhase(s.to_string()))?;
			Ok(ProjectPhase::Development { iteration })
		} else {
			match s {
				"exploration" => Ok(ProjectPhase::Exploration),
				"refinement" => Ok(ProjectPhase::Refinement),
				"verification" => Ok(ProjectPhase::Verification),
				"complete" => Ok(ProjectPhase::Complete),
				_ => Err(crate::error::MultiAgentError::InvalidProjectPhase(
					s.to_string(),
				)),
			}
		}
	}
}

/// Status of a project run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
	/// Project is initializing
	#[default]
	Initializing,
	/// Project is running
	Running,
	/// Project is paused
	Paused,
	/// Project completed successfully
	Completed,
	/// Project failed
	Failed,
	/// Project was cancelled
	Cancelled,
}

impl std::fmt::Display for ProjectStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ProjectStatus::Initializing => write!(f, "initializing"),
			ProjectStatus::Running => write!(f, "running"),
			ProjectStatus::Paused => write!(f, "paused"),
			ProjectStatus::Completed => write!(f, "completed"),
			ProjectStatus::Failed => write!(f, "failed"),
			ProjectStatus::Cancelled => write!(f, "cancelled"),
		}
	}
}

impl std::str::FromStr for ProjectStatus {
	type Err = crate::error::MultiAgentError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"initializing" => Ok(ProjectStatus::Initializing),
			"running" => Ok(ProjectStatus::Running),
			"paused" => Ok(ProjectStatus::Paused),
			"completed" => Ok(ProjectStatus::Completed),
			"failed" => Ok(ProjectStatus::Failed),
			"cancelled" => Ok(ProjectStatus::Cancelled),
			_ => Err(crate::error::MultiAgentError::InvalidProjectPhase(
				s.to_string(),
			)),
		}
	}
}

/// Metrics for a project run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ProjectMetrics {
	/// Total tasks created
	pub tasks_created: u64,
	/// Tasks completed successfully
	pub tasks_completed: u64,
	/// Tasks failed
	pub tasks_failed: u64,
	/// Tasks cancelled
	pub tasks_cancelled: u64,
	/// Lines of code added
	pub lines_added: u64,
	/// Lines of code removed
	pub lines_removed: u64,
	/// Test pass rate (0.0 to 1.0)
	pub test_pass_rate: f64,
	/// Average task duration in milliseconds
	pub avg_task_duration_ms: u64,
	/// Active workers count
	pub active_workers: u32,
	/// Active planners count
	pub active_planners: u32,
}

/// Metrics for a single domain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DomainMetrics {
	/// Tasks completed
	pub tasks_completed: u64,
	/// Tasks failed
	pub tasks_failed: u64,
	/// Lines added
	pub lines_added: u64,
	/// Lines removed
	pub lines_removed: u64,
	/// Average task duration in milliseconds
	pub avg_task_duration_ms: u64,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_project_state_new() {
		let state = ProjectState::new(
			"org-123".to_string(),
			"/path/to/project".to_string(),
			"Implement feature X".to_string(),
			"user-123".to_string(),
		);
		assert_eq!(state.org_id, "org-123");
		assert_eq!(state.phase, ProjectPhase::Exploration);
		assert_eq!(state.status, ProjectStatus::Running);
	}

	#[test]
	fn test_project_phase_display() {
		assert_eq!(ProjectPhase::Exploration.to_string(), "exploration");
		assert_eq!(
			ProjectPhase::Development { iteration: 3 }.to_string(),
			"development:3"
		);
	}

	#[test]
	fn test_project_phase_parse() {
		assert_eq!(
			"exploration".parse::<ProjectPhase>().unwrap(),
			ProjectPhase::Exploration
		);
		assert_eq!(
			"development:5".parse::<ProjectPhase>().unwrap(),
			ProjectPhase::Development { iteration: 5 }
		);
	}

	#[test]
	fn test_project_status_display() {
		assert_eq!(ProjectStatus::Running.to_string(), "running");
		assert_eq!(ProjectStatus::Paused.to_string(), "paused");
	}

	#[test]
	fn test_project_status_parse() {
		assert_eq!(
			"running".parse::<ProjectStatus>().unwrap(),
			ProjectStatus::Running
		);
		assert_eq!(
			"completed".parse::<ProjectStatus>().unwrap(),
			ProjectStatus::Completed
		);
	}
}
