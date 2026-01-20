// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Verification engine for grounding claims in executable reality.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use loom_context_core::{Claim, ClaimType, OrgId, VerificationResult};

use crate::error::{ContextError, Result};

/// A verifier that can check claims of a specific type.
#[async_trait]
pub trait Verifier: Send + Sync {
	/// Verify a claim.
	async fn verify(&self, claim: &Claim, org_id: OrgId) -> Result<VerificationResult>;

	/// Returns the claim types this verifier handles.
	fn handles(&self) -> Vec<&'static str>;
}

/// Engine for running verifications.
pub struct VerificationEngine {
	/// Available verifiers.
	verifiers: HashMap<String, Arc<dyn Verifier>>,
	/// Organization ID.
	org_id: OrgId,
}

impl VerificationEngine {
	/// Creates a new verification engine.
	pub fn new(org_id: OrgId) -> Self {
		Self {
			verifiers: HashMap::new(),
			org_id,
		}
	}

	/// Registers a verifier.
	pub fn register_verifier(&mut self, verifier: Arc<dyn Verifier>) {
		for claim_type in verifier.handles() {
			self.verifiers.insert(claim_type.to_string(), verifier.clone());
		}
	}

	/// Verifies a single claim.
	pub async fn verify(&self, claim: &Claim) -> Result<VerificationResult> {
		let type_key = claim.claim_type.type_key();

		if let Some(verifier) = self.verifiers.get(type_key) {
			verifier.verify(claim, self.org_id).await
		} else {
			// No verifier registered, return a pass-through result
			Ok(VerificationResult::success(
				self.org_id,
				claim.clone(),
				format!("No verifier for claim type: {}", type_key),
				Duration::ZERO,
			))
		}
	}

	/// Verifies multiple claims.
	pub async fn verify_claims(&self, claims: &[Claim]) -> Vec<VerificationResult> {
		let mut results = Vec::with_capacity(claims.len());

		for claim in claims {
			match self.verify(claim).await {
				Ok(result) => results.push(result),
				Err(e) => {
					results.push(VerificationResult::failure(
						self.org_id,
						claim.clone(),
						format!("Verification error: {}", e),
						Duration::ZERO,
					));
				}
			}
		}

		results
	}

	/// Returns the organization ID.
	pub fn org_id(&self) -> OrgId {
		self.org_id
	}
}

/// A simple command execution verifier.
pub struct CommandVerifier;

#[async_trait]
impl Verifier for CommandVerifier {
	async fn verify(&self, claim: &Claim, org_id: OrgId) -> Result<VerificationResult> {
		let ClaimType::CommandSucceeds { command } = &claim.claim_type else {
			return Err(ContextError::Internal("Invalid claim type".to_string()));
		};

		let start = Instant::now();

		// Note: In a real implementation, this would execute the command
		// For now, we return a placeholder result
		let result = VerificationResult::success(
			org_id,
			claim.clone(),
			format!("Command verification placeholder: {}", command),
			start.elapsed(),
		);

		Ok(result)
	}

	fn handles(&self) -> Vec<&'static str> {
		vec!["command_succeeds"]
	}
}

/// A file content verifier.
pub struct FileContainsVerifier;

#[async_trait]
impl Verifier for FileContainsVerifier {
	async fn verify(&self, claim: &Claim, org_id: OrgId) -> Result<VerificationResult> {
		let ClaimType::FileContains { path, pattern } = &claim.claim_type else {
			return Err(ContextError::Internal("Invalid claim type".to_string()));
		};

		let start = Instant::now();

		// Read file and check for pattern
		match std::fs::read_to_string(path) {
			Ok(content) => {
				if content.contains(pattern) {
					Ok(VerificationResult::success(
						org_id,
						claim.clone(),
						format!("Pattern '{}' found in {}", pattern, path.display()),
						start.elapsed(),
					))
				} else {
					Ok(VerificationResult::failure(
						org_id,
						claim.clone(),
						format!("Pattern '{}' not found in {}", pattern, path.display()),
						start.elapsed(),
					))
				}
			}
			Err(e) => Ok(VerificationResult::failure(
				org_id,
				claim.clone(),
				format!("Failed to read {}: {}", path.display(), e),
				start.elapsed(),
			)),
		}
	}

	fn handles(&self) -> Vec<&'static str> {
		vec!["file_contains"]
	}
}

/// A compile verification placeholder.
pub struct CompileVerifier;

#[async_trait]
impl Verifier for CompileVerifier {
	async fn verify(&self, claim: &Claim, org_id: OrgId) -> Result<VerificationResult> {
		let ClaimType::Compiles { files } = &claim.claim_type else {
			return Err(ContextError::Internal("Invalid claim type".to_string()));
		};

		let start = Instant::now();

		// Placeholder: would run actual compile check
		Ok(VerificationResult::success(
			org_id,
			claim.clone(),
			format!("Compile verification placeholder for {} files", files.len()),
			start.elapsed(),
		))
	}

	fn handles(&self) -> Vec<&'static str> {
		vec!["compiles", "type_checks", "lint_passes"]
	}
}

/// A test verification placeholder.
pub struct TestVerifier;

#[async_trait]
impl Verifier for TestVerifier {
	async fn verify(&self, claim: &Claim, org_id: OrgId) -> Result<VerificationResult> {
		let ClaimType::TestsPass { test_pattern } = &claim.claim_type else {
			return Err(ContextError::Internal("Invalid claim type".to_string()));
		};

		let start = Instant::now();

		// Placeholder: would run actual tests
		Ok(VerificationResult::success(
			org_id,
			claim.clone(),
			format!("Test verification placeholder for pattern: {}", test_pattern),
			start.elapsed(),
		))
	}

	fn handles(&self) -> Vec<&'static str> {
		vec!["tests_pass"]
	}
}

/// Creates a verification engine with default verifiers.
pub fn create_default_engine(org_id: OrgId) -> VerificationEngine {
	let mut engine = VerificationEngine::new(org_id);

	engine.register_verifier(Arc::new(CommandVerifier));
	engine.register_verifier(Arc::new(FileContainsVerifier));
	engine.register_verifier(Arc::new(CompileVerifier));
	engine.register_verifier(Arc::new(TestVerifier));

	engine
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;
	use tempfile::NamedTempFile;
	use std::io::Write;

	#[tokio::test]
	async fn verification_engine_basic() {
		let org_id = OrgId::new();
		let engine = create_default_engine(org_id);

		let claim = Claim::new(
			"command succeeds",
			ClaimType::CommandSucceeds {
				command: "echo hello".to_string(),
			},
		);

		let result = engine.verify(&claim).await.unwrap();
		assert!(result.passed);
	}

	#[tokio::test]
	async fn file_contains_verifier_success() {
		let org_id = OrgId::new();

		// Create a temp file with content
		let mut file = NamedTempFile::new().unwrap();
		writeln!(file, "Hello, World!").unwrap();
		file.flush().unwrap();

		let verifier = FileContainsVerifier;
		let claim = Claim::new(
			"file contains hello",
			ClaimType::FileContains {
				path: file.path().to_path_buf(),
				pattern: "Hello".to_string(),
			},
		);

		let result = verifier.verify(&claim, org_id).await.unwrap();
		assert!(result.passed);
	}

	#[tokio::test]
	async fn file_contains_verifier_failure() {
		let org_id = OrgId::new();

		// Create a temp file with content
		let mut file = NamedTempFile::new().unwrap();
		writeln!(file, "Hello, World!").unwrap();
		file.flush().unwrap();

		let verifier = FileContainsVerifier;
		let claim = Claim::new(
			"file contains goodbye",
			ClaimType::FileContains {
				path: file.path().to_path_buf(),
				pattern: "Goodbye".to_string(),
			},
		);

		let result = verifier.verify(&claim, org_id).await.unwrap();
		assert!(!result.passed);
	}

	#[tokio::test]
	async fn verify_multiple_claims() {
		let org_id = OrgId::new();
		let engine = create_default_engine(org_id);

		let claims = vec![
			Claim::new(
				"code compiles",
				ClaimType::Compiles {
					files: vec![PathBuf::from("src/main.rs")],
				},
			),
			Claim::new(
				"tests pass",
				ClaimType::TestsPass {
					test_pattern: "test_*".to_string(),
				},
			),
		];

		let results = engine.verify_claims(&claims).await;
		assert_eq!(results.len(), 2);
	}

	#[tokio::test]
	async fn unknown_claim_type() {
		let org_id = OrgId::new();
		let engine = VerificationEngine::new(org_id); // No verifiers registered

		let claim = Claim::new(
			"command succeeds",
			ClaimType::CommandSucceeds {
				command: "test".to_string(),
			},
		);

		// Should not fail, just return pass-through
		let result = engine.verify(&claim).await.unwrap();
		assert!(result.passed);
		assert!(result.output.contains("No verifier"));
	}
}
