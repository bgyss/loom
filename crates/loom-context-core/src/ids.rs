// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Identifier types for the context management system.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a memory item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for MemoryId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for MemoryId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for MemoryId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

/// Unique identifier for a context snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextSnapshotId(pub Uuid);

impl ContextSnapshotId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for ContextSnapshotId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for ContextSnapshotId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for ContextSnapshotId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

/// Unique identifier for a verification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VerificationId(pub Uuid);

impl VerificationId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for VerificationId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for VerificationId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for VerificationId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

/// Unique identifier for a drift signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DriftSignalId(pub Uuid);

impl DriftSignalId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for DriftSignalId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for DriftSignalId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for DriftSignalId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

/// Unique identifier for an organization (re-exported for convenience).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrgId(pub Uuid);

impl OrgId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for OrgId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for OrgId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for OrgId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	#[test]
	fn memory_id_roundtrip() {
		let id = MemoryId::new();
		let s = id.to_string();
		let parsed: MemoryId = s.parse().unwrap();
		assert_eq!(id, parsed);
	}

	#[test]
	fn context_snapshot_id_roundtrip() {
		let id = ContextSnapshotId::new();
		let s = id.to_string();
		let parsed: ContextSnapshotId = s.parse().unwrap();
		assert_eq!(id, parsed);
	}

	#[test]
	fn verification_id_roundtrip() {
		let id = VerificationId::new();
		let s = id.to_string();
		let parsed: VerificationId = s.parse().unwrap();
		assert_eq!(id, parsed);
	}

	#[test]
	fn drift_signal_id_roundtrip() {
		let id = DriftSignalId::new();
		let s = id.to_string();
		let parsed: DriftSignalId = s.parse().unwrap();
		assert_eq!(id, parsed);
	}

	proptest! {
		#[test]
		fn memory_id_is_unique(_seed: u64) {
			let id1 = MemoryId::new();
			let id2 = MemoryId::new();
			prop_assert_ne!(id1, id2);
		}

		#[test]
		fn context_snapshot_id_is_unique(_seed: u64) {
			let id1 = ContextSnapshotId::new();
			let id2 = ContextSnapshotId::new();
			prop_assert_ne!(id1, id2);
		}

		#[test]
		fn verification_id_is_unique(_seed: u64) {
			let id1 = VerificationId::new();
			let id2 = VerificationId::new();
			prop_assert_ne!(id1, id2);
		}

		#[test]
		fn drift_signal_id_is_unique(_seed: u64) {
			let id1 = DriftSignalId::new();
			let id2 = DriftSignalId::new();
			prop_assert_ne!(id1, id2);
		}
	}
}
