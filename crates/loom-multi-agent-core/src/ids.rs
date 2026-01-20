// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Identifier types for the multi-agent system.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
	($name:ident, $prefix:literal) => {
		#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
		#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
		pub struct $name(pub Uuid);

		impl $name {
			#[must_use]
			pub fn new() -> Self {
				Self(Uuid::now_v7())
			}

			#[must_use]
			pub fn as_uuid(&self) -> &Uuid {
				&self.0
			}
		}

		impl Default for $name {
			fn default() -> Self {
				Self::new()
			}
		}

		impl std::fmt::Display for $name {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(f, "{}-{}", $prefix, self.0)
			}
		}

		impl std::str::FromStr for $name {
			type Err = uuid::Error;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				let s = s.strip_prefix(concat!($prefix, "-")).unwrap_or(s);
				Ok(Self(Uuid::parse_str(s)?))
			}
		}
	};
}

define_id!(TaskId, "task");
define_id!(PlannerId, "planner");
define_id!(WorkerId, "worker");
define_id!(DomainId, "domain");
define_id!(ProjectRunId, "run");
define_id!(CheckpointId, "checkpoint");

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_task_id_new() {
		let id = TaskId::new();
		assert!(!id.to_string().is_empty());
		assert!(id.to_string().starts_with("task-"));
	}

	#[test]
	fn test_task_id_parse() {
		let id = TaskId::new();
		let parsed: TaskId = id.to_string().parse().unwrap();
		assert_eq!(id, parsed);
	}

	#[test]
	fn test_worker_id_display() {
		let id = WorkerId::new();
		assert!(id.to_string().starts_with("worker-"));
	}

	#[test]
	fn test_planner_id_display() {
		let id = PlannerId::new();
		assert!(id.to_string().starts_with("planner-"));
	}

	#[test]
	fn test_domain_id_display() {
		let id = DomainId::new();
		assert!(id.to_string().starts_with("domain-"));
	}

	#[test]
	fn test_project_run_id_display() {
		let id = ProjectRunId::new();
		assert!(id.to_string().starts_with("run-"));
	}

	#[test]
	fn test_checkpoint_id_display() {
		let id = CheckpointId::new();
		assert!(id.to_string().starts_with("checkpoint-"));
	}
}
