// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Multi-agent coordination system implementation.
//!
//! This crate provides the agent implementations for the hierarchical
//! multi-agent coordination system:
//!
//! - [`Orchestrator`] - Singleton coordinator for global state
//! - [`DomainPlanner`] - Area expert that creates and manages tasks
//! - [`Worker`] - Task executor
//! - [`JudgeAgent`] - Evaluates completions
//! - [`ModelSelector`] - Role-based model selection

pub mod judge;
pub mod model_selector;
pub mod orchestrator;
pub mod planner;
pub mod worker;
pub mod worker_pool;

pub use judge::JudgeAgent;
pub use model_selector::ModelSelector;
pub use orchestrator::Orchestrator;
pub use planner::DomainPlanner;
pub use worker::Worker;
pub use worker_pool::WorkerPool;

/// Result type for multi-agent operations.
pub type Result<T> = std::result::Result<T, anyhow::Error>;
