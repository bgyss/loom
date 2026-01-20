// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Context management logic and services for Loom agents.
//!
//! This crate provides the core logic for maintaining agent context integrity
//! over extended operations, including memory management, drift detection,
//! verification, and strategy selection.
//!
//! # Overview
//!
//! The context management system consists of:
//! - [`MemoryStore`]: In-memory storage with decay and consolidation
//! - [`ContextAssembler`]: Priority-based context window assembly
//! - [`DriftDetector`]: Detection of context degradation patterns
//! - [`DriftCorrector`]: Correction strategies for detected drift
//! - [`VerificationEngine`]: Grounding claims in executable reality
//! - [`FeedbackIntegrator`]: Learning from verification results
//! - [`StrategySelector`]: Dynamic reasoning strategy selection
//! - [`ContextManager`]: Main facade combining all components
//!
//! # Example
//!
//! ```
//! use loom_context::{ContextManager, AgentAction};
//! use loom_context_core::{Claim, ClaimType, ResourceUsage, OrgId};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a context manager
//! let org_id = OrgId::new();
//! let mut manager = ContextManager::new(org_id);
//!
//! // Select a strategy for the task
//! let usage = ResourceUsage::new();
//! let strategy = manager.select_strategy("fix the login bug", &usage);
//!
//! // Assemble context
//! let context = manager.assemble("task-1", "Fix login validation", &usage).await?;
//!
//! // Record an action
//! manager.record_action(AgentAction::new("edit").with_topics(vec!["auth"]));
//!
//! // Detect any drift
//! let drift_signals = manager.detect_drift();
//!
//! // Verify a claim
//! let claim = Claim::new("tests pass", ClaimType::TestsPass {
//!     test_pattern: "test_*".to_string(),
//! });
//! let result = manager.verify(&claim).await?;
//!
//! // Integrate feedback
//! let summary = manager.absorb_learnings(&[result]).await?;
//! # Ok(())
//! # }
//! ```

pub mod assembler;
pub mod drift_corrector;
pub mod drift_detector;
pub mod error;
pub mod feedback;
pub mod manager;
pub mod persistent_strategy;
pub mod resource_prompt;
pub mod store;
pub mod strategy;
pub mod verification;

// Re-export commonly used types
pub use assembler::ContextAssembler;
pub use drift_corrector::{CorrectionReport, DriftCorrector};
pub use drift_detector::{AgentAction, DriftDetector};
pub use error::{ContextError, Result};
pub use feedback::FeedbackIntegrator;
pub use manager::ContextManager;
pub use persistent_strategy::{PersistentStrategySelector, StrategyPerformanceStore};
pub use resource_prompt::ResourceAwarePromptBuilder;
pub use store::{DecayConfig, MemoryStore};
pub use strategy::{StrategyFeedbackMonitor, StrategyPromptBuilder, StrategySelector, TaskClassifier};
pub use verification::{
	create_default_engine, CommandVerifier, CompileVerifier, FileContainsVerifier, TestVerifier,
	VerificationEngine, Verifier,
};
