// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Core types for the Loom multi-agent coordination system.
//!
//! This crate provides shared types for the hierarchical multi-agent system
//! that enables 10-100+ concurrent agents to work on a single codebase.
//!
//! ## Architecture
//!
//! The system uses a hierarchical role-based architecture:
//! - **Orchestrator**: Singleton coordinator for global state
//! - **Domain Planners**: Area experts that create and manage tasks
//! - **Workers**: Task executors that wrap the core Agent
//! - **Judge Agent**: Evaluates completions and determines iteration
//!
//! ## Key Types
//!
//! - [`Task`] - A unit of work with specification and status
//! - [`TaskCompletion`] - Result of task execution
//! - [`Domain`] - An area of the codebase
//! - [`ProjectState`] - Global project run state
//! - [`Verdict`] - Judge evaluation result

pub mod completion;
pub mod config;
pub mod domain;
pub mod error;
pub mod ids;
pub mod project;
pub mod task;
pub mod verdict;

pub use completion::{
	Artifact, FileChange, Learning, TaskCompletion, TaskOutput, TaskProgress, TaskSuggestion,
	VerificationResult,
};
pub use config::{AgentRole, EvaluationCriteria, ModelConfig, ProjectConfig};
pub use domain::{Domain, DomainState};
pub use error::MultiAgentError;
pub use ids::{CheckpointId, DomainId, PlannerId, ProjectRunId, TaskId, WorkerId};
pub use project::{DomainMetrics, ProjectMetrics, ProjectPhase, ProjectState, ProjectStatus};
pub use task::{
	ContextItem, ContextSnapshot, Task, TaskSpecification, TaskStatus, TaskType,
	VerifiableCriterion, VerificationMethod,
};
pub use verdict::Verdict;

/// Result type for multi-agent operations.
pub type Result<T> = std::result::Result<T, MultiAgentError>;
