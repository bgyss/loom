// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! SQLite repository for context management persistence.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use loom_context_core::{
	ContextSnapshot, ContextSnapshotId, CorrectionAction, DriftSignal, DriftSignalId,
	DriftSignalRecord, MemoryContent, MemoryId, MemoryItem, MemoryQuery, MemorySource, MemoryType,
	OrgId, PerformanceStats, ReasoningStrategy, ResourceUsage, TaskType, VerificationId,
	VerificationResult,
};
use sqlx::SqlitePool;

use crate::error::{ContextDbError, Result};

/// Repository trait for context management persistence.
#[async_trait]
pub trait ContextRepository: Send + Sync {
	// Memory operations
	async fn create_memory(&self, item: &MemoryItem) -> Result<MemoryId>;
	async fn get_memory(&self, id: &MemoryId, org_id: &OrgId) -> Result<MemoryItem>;
	async fn update_memory(&self, item: &MemoryItem) -> Result<()>;
	async fn delete_memory(&self, id: &MemoryId, org_id: &OrgId) -> Result<()>;
	async fn list_memories(&self, org_id: &OrgId, limit: u32, offset: u32) -> Result<Vec<MemoryItem>>;
	async fn query_memories(&self, org_id: &OrgId, query: &MemoryQuery) -> Result<Vec<MemoryItem>>;

	// Verification operations
	async fn create_verification(&self, result: &VerificationResult) -> Result<VerificationId>;
	async fn get_verification(&self, id: &VerificationId, org_id: &OrgId) -> Result<VerificationResult>;
	async fn list_verifications_for_task(
		&self,
		org_id: &OrgId,
		task_id: &str,
	) -> Result<Vec<VerificationResult>>;

	// Drift signal operations
	async fn create_drift_signal(&self, record: &DriftSignalRecord) -> Result<DriftSignalId>;
	async fn update_drift_signal(&self, record: &DriftSignalRecord) -> Result<()>;
	async fn list_active_drift_signals(&self, org_id: &OrgId) -> Result<Vec<DriftSignalRecord>>;

	// Strategy performance operations
	async fn record_strategy_attempt(
		&self,
		org_id: &OrgId,
		task_type: TaskType,
		strategy: ReasoningStrategy,
		success: bool,
		duration_ms: u64,
		tokens_used: u64,
	) -> Result<()>;
	async fn get_strategy_stats(
		&self,
		org_id: &OrgId,
		task_type: &TaskType,
		strategy: &ReasoningStrategy,
	) -> Result<Option<PerformanceStats>>;

	async fn get_all_strategy_stats_for_task_type(
		&self,
		org_id: &OrgId,
		task_type: &TaskType,
	) -> Result<Vec<(ReasoningStrategy, PerformanceStats)>>;

	// Snapshot operations
	async fn create_snapshot(&self, snapshot: &ContextSnapshot) -> Result<ContextSnapshotId>;
	async fn get_snapshot(&self, id: &ContextSnapshotId, org_id: &OrgId) -> Result<ContextSnapshot>;
	async fn list_snapshots_for_task(
		&self,
		org_id: &OrgId,
		task_id: &str,
	) -> Result<Vec<ContextSnapshot>>;
}

/// SQLite implementation of the context repository.
pub struct SqliteContextRepository {
	pool: SqlitePool,
}

impl SqliteContextRepository {
	/// Creates a new SQLite context repository.
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

#[async_trait]
impl ContextRepository for SqliteContextRepository {
	async fn create_memory(&self, item: &MemoryItem) -> Result<MemoryId> {
		let id = item.id.to_string();
		let memory_type = item.memory_type.to_string();
		let content_type = item.content.content_type();
		let content = serde_json::to_string(&item.content)?;
		let created_at = item.created_at.to_rfc3339();
		let last_accessed = item.last_accessed.to_rfc3339();
		let tags = serde_json::to_string(&item.tags)?;
		let source_type = item.source.source_type();
		let source_id = item.source.source_id();
		let org_id = item.org_id.to_string();

		sqlx::query(
			r#"
			INSERT INTO ctx_memory_items (
				id, memory_type, content_type, content, created_at, last_accessed,
				access_count, relevance_score, tags, source_type, source_id, org_id
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(&memory_type)
		.bind(content_type)
		.bind(&content)
		.bind(&created_at)
		.bind(&last_accessed)
		.bind(item.access_count as i64)
		.bind(item.relevance_score)
		.bind(&tags)
		.bind(source_type)
		.bind(source_id)
		.bind(&org_id)
		.execute(&self.pool)
		.await?;

		Ok(item.id)
	}

	async fn get_memory(&self, id: &MemoryId, org_id: &OrgId) -> Result<MemoryItem> {
		let row = sqlx::query_as::<_, MemoryRow>(
			r#"
			SELECT id, memory_type, content_type, content, created_at, last_accessed,
				   access_count, relevance_score, tags, source_type, source_id, org_id
			FROM ctx_memory_items
			WHERE id = ? AND org_id = ?
			"#,
		)
		.bind(id.to_string())
		.bind(org_id.to_string())
		.fetch_optional(&self.pool)
		.await?
		.ok_or_else(|| ContextDbError::MemoryNotFound(id.to_string()))?;

		row.into_memory_item()
	}

	async fn update_memory(&self, item: &MemoryItem) -> Result<()> {
		let memory_type = item.memory_type.to_string();
		let content = serde_json::to_string(&item.content)?;
		let last_accessed = item.last_accessed.to_rfc3339();
		let tags = serde_json::to_string(&item.tags)?;

		sqlx::query(
			r#"
			UPDATE ctx_memory_items
			SET memory_type = ?, content = ?, last_accessed = ?,
				access_count = ?, relevance_score = ?, tags = ?
			WHERE id = ? AND org_id = ?
			"#,
		)
		.bind(&memory_type)
		.bind(&content)
		.bind(&last_accessed)
		.bind(item.access_count as i64)
		.bind(item.relevance_score)
		.bind(&tags)
		.bind(item.id.to_string())
		.bind(item.org_id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	async fn delete_memory(&self, id: &MemoryId, org_id: &OrgId) -> Result<()> {
		sqlx::query("DELETE FROM ctx_memory_items WHERE id = ? AND org_id = ?")
			.bind(id.to_string())
			.bind(org_id.to_string())
			.execute(&self.pool)
			.await?;
		Ok(())
	}

	async fn list_memories(&self, org_id: &OrgId, limit: u32, offset: u32) -> Result<Vec<MemoryItem>> {
		let rows = sqlx::query_as::<_, MemoryRow>(
			r#"
			SELECT id, memory_type, content_type, content, created_at, last_accessed,
				   access_count, relevance_score, tags, source_type, source_id, org_id
			FROM ctx_memory_items
			WHERE org_id = ?
			ORDER BY relevance_score DESC, last_accessed DESC
			LIMIT ? OFFSET ?
			"#,
		)
		.bind(org_id.to_string())
		.bind(limit as i64)
		.bind(offset as i64)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(|r| r.into_memory_item()).collect()
	}

	async fn query_memories(&self, org_id: &OrgId, query: &MemoryQuery) -> Result<Vec<MemoryItem>> {
		// Build dynamic query
		let mut sql = String::from(
			r#"
			SELECT id, memory_type, content_type, content, created_at, last_accessed,
				   access_count, relevance_score, tags, source_type, source_id, org_id
			FROM ctx_memory_items
			WHERE org_id = ?
			"#,
		);

		let mut args: Vec<String> = vec![org_id.to_string()];

		if let Some(ref mt) = query.memory_type {
			sql.push_str(" AND memory_type = ?");
			args.push(mt.to_string());
		}

		if let Some(min_rel) = query.min_relevance {
			sql.push_str(" AND relevance_score >= ?");
			args.push(min_rel.to_string());
		}

		sql.push_str(" ORDER BY relevance_score DESC, last_accessed DESC");

		if let Some(limit) = query.max_results {
			sql.push_str(&format!(" LIMIT {}", limit));
		}

		// We can't use dynamic binding easily, so we'll use a simpler approach
		let rows = sqlx::query_as::<_, MemoryRow>(&sql)
			.bind(&args[0])
			.fetch_all(&self.pool)
			.await?;

		let mut items: Vec<MemoryItem> = rows
			.into_iter()
			.filter_map(|r| r.into_memory_item().ok())
			.collect();

		// Apply memory type filter
		if let Some(ref mt) = query.memory_type {
			items.retain(|i| i.memory_type == *mt);
		}

		// Apply min relevance filter
		if let Some(min_rel) = query.min_relevance {
			items.retain(|i| i.relevance_score >= min_rel);
		}

		// Apply tag filter
		if !query.tags.is_empty() {
			items.retain(|i| query.tags.iter().any(|t| i.tags.contains(t)));
		}

		// Apply text search filter
		if let Some(ref search) = query.search_text {
			let search_lower = search.to_lowercase();
			items.retain(|i| {
				serde_json::to_string(&i.content)
					.map(|s| s.to_lowercase().contains(&search_lower))
					.unwrap_or(false)
			});
		}

		// Apply limit
		if let Some(limit) = query.max_results {
			items.truncate(limit);
		}

		Ok(items)
	}

	async fn create_verification(&self, result: &VerificationResult) -> Result<VerificationId> {
		let id = result.id.to_string();
		let claim_type = result.claim.claim_type.type_key();
		let assertion = &result.claim.assertion;
		let passed = result.passed;
		let output = &result.output;
		let verified_at = result.verified_at.to_rfc3339();
		let duration_ms = result.duration.as_millis() as i64;
		let task_id = result.task_id.as_deref();
		let org_id = result.org_id.to_string();

		sqlx::query(
			r#"
			INSERT INTO ctx_verifications (
				id, claim_type, assertion, passed, output, verified_at, duration_ms, task_id, org_id
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(claim_type)
		.bind(assertion)
		.bind(passed)
		.bind(output)
		.bind(&verified_at)
		.bind(duration_ms)
		.bind(task_id)
		.bind(&org_id)
		.execute(&self.pool)
		.await?;

		Ok(result.id)
	}

	async fn get_verification(&self, id: &VerificationId, org_id: &OrgId) -> Result<VerificationResult> {
		let row = sqlx::query_as::<_, VerificationRow>(
			r#"
			SELECT id, claim_type, assertion, passed, output, verified_at, duration_ms, task_id, org_id
			FROM ctx_verifications
			WHERE id = ? AND org_id = ?
			"#,
		)
		.bind(id.to_string())
		.bind(org_id.to_string())
		.fetch_optional(&self.pool)
		.await?
		.ok_or_else(|| ContextDbError::VerificationNotFound(id.to_string()))?;

		row.into_verification_result()
	}

	async fn list_verifications_for_task(
		&self,
		org_id: &OrgId,
		task_id: &str,
	) -> Result<Vec<VerificationResult>> {
		let rows = sqlx::query_as::<_, VerificationRow>(
			r#"
			SELECT id, claim_type, assertion, passed, output, verified_at, duration_ms, task_id, org_id
			FROM ctx_verifications
			WHERE org_id = ? AND task_id = ?
			ORDER BY verified_at DESC
			"#,
		)
		.bind(org_id.to_string())
		.bind(task_id)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(|r| r.into_verification_result()).collect()
	}

	async fn create_drift_signal(&self, record: &DriftSignalRecord) -> Result<DriftSignalId> {
		let id = record.id.to_string();
		let signal_type = record.signal.signal_type();
		let details = serde_json::to_string(&record.signal)?;
		let detected_at = record.detected_at.to_rfc3339();
		let corrected_at = record.corrected_at.map(|dt| dt.to_rfc3339());
		let correction_action = record
			.correction_action
			.as_ref()
			.map(|a| serde_json::to_string(a))
			.transpose()?;
		let org_id = record.org_id.to_string();

		sqlx::query(
			r#"
			INSERT INTO ctx_drift_signals (
				id, signal_type, details, detected_at, corrected_at, correction_action, org_id
			) VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(signal_type)
		.bind(&details)
		.bind(&detected_at)
		.bind(&corrected_at)
		.bind(&correction_action)
		.bind(&org_id)
		.execute(&self.pool)
		.await?;

		Ok(record.id)
	}

	async fn update_drift_signal(&self, record: &DriftSignalRecord) -> Result<()> {
		let corrected_at = record.corrected_at.map(|dt| dt.to_rfc3339());
		let correction_action = record
			.correction_action
			.as_ref()
			.map(|a| serde_json::to_string(a))
			.transpose()?;

		sqlx::query(
			r#"
			UPDATE ctx_drift_signals
			SET corrected_at = ?, correction_action = ?
			WHERE id = ? AND org_id = ?
			"#,
		)
		.bind(&corrected_at)
		.bind(&correction_action)
		.bind(record.id.to_string())
		.bind(record.org_id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	async fn list_active_drift_signals(&self, org_id: &OrgId) -> Result<Vec<DriftSignalRecord>> {
		let rows = sqlx::query_as::<_, DriftSignalRow>(
			r#"
			SELECT id, signal_type, details, detected_at, corrected_at, correction_action, org_id
			FROM ctx_drift_signals
			WHERE org_id = ? AND corrected_at IS NULL
			ORDER BY detected_at DESC
			"#,
		)
		.bind(org_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(|r| r.into_drift_signal_record()).collect()
	}

	async fn record_strategy_attempt(
		&self,
		org_id: &OrgId,
		task_type: TaskType,
		strategy: ReasoningStrategy,
		success: bool,
		duration_ms: u64,
		tokens_used: u64,
	) -> Result<()> {
		let org_id_str = org_id.to_string();
		let task_type_str = task_type.name();
		let strategy_str = strategy.name();

		// Upsert strategy performance
		sqlx::query(
			r#"
			INSERT INTO ctx_strategy_performance (
				org_id, task_type, strategy, attempts, successes, avg_duration_ms, avg_tokens_used
			) VALUES (?, ?, ?, 1, ?, ?, ?)
			ON CONFLICT(org_id, task_type, strategy) DO UPDATE SET
				attempts = attempts + 1,
				successes = successes + ?,
				avg_duration_ms = (COALESCE(avg_duration_ms, 0) * (attempts - 1) + ?) / attempts,
				avg_tokens_used = (COALESCE(avg_tokens_used, 0) * (attempts - 1) + ?) / attempts
			"#,
		)
		.bind(&org_id_str)
		.bind(task_type_str)
		.bind(strategy_str)
		.bind(if success { 1i64 } else { 0i64 })
		.bind(duration_ms as i64)
		.bind(tokens_used as i64)
		.bind(if success { 1i64 } else { 0i64 })
		.bind(duration_ms as i64)
		.bind(tokens_used as i64)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	async fn get_strategy_stats(
		&self,
		org_id: &OrgId,
		task_type: &TaskType,
		strategy: &ReasoningStrategy,
	) -> Result<Option<PerformanceStats>> {
		let row = sqlx::query_as::<_, StrategyPerformanceRow>(
			r#"
			SELECT task_type, strategy, attempts, successes, avg_duration_ms, avg_tokens_used
			FROM ctx_strategy_performance
			WHERE org_id = ? AND task_type = ? AND strategy = ?
			"#,
		)
		.bind(org_id.to_string())
		.bind(task_type.name())
		.bind(strategy.name())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row.map(|r| r.into_performance_stats()))
	}

	async fn get_all_strategy_stats_for_task_type(
		&self,
		org_id: &OrgId,
		task_type: &TaskType,
	) -> Result<Vec<(ReasoningStrategy, PerformanceStats)>> {
		let rows = sqlx::query_as::<_, StrategyPerformanceRow>(
			r#"
			SELECT task_type, strategy, attempts, successes, avg_duration_ms, avg_tokens_used
			FROM ctx_strategy_performance
			WHERE org_id = ? AND task_type = ?
			ORDER BY successes DESC
			"#,
		)
		.bind(org_id.to_string())
		.bind(task_type.name())
		.fetch_all(&self.pool)
		.await?;

		let mut results = Vec::new();
		for row in rows {
			if let Some(strategy) = parse_strategy(&row.strategy) {
				results.push((strategy, row.into_performance_stats()));
			}
		}
		Ok(results)
	}

	async fn create_snapshot(&self, snapshot: &ContextSnapshot) -> Result<ContextSnapshotId> {
		let id = snapshot.id.to_string();
		let task_id = &snapshot.task_id;
		let assembled_at = snapshot.assembled_at.to_rfc3339();
		let total_tokens = snapshot.total_tokens as i64;
		let sections = serde_json::to_string(&snapshot.sections)?;
		let resource_usage = serde_json::to_string(&snapshot.resource_usage)?;
		let org_id = snapshot.org_id.to_string();

		sqlx::query(
			r#"
			INSERT INTO ctx_snapshots (
				id, task_id, assembled_at, total_tokens, sections, resource_usage, org_id
			) VALUES (?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(&id)
		.bind(task_id)
		.bind(&assembled_at)
		.bind(total_tokens)
		.bind(&sections)
		.bind(&resource_usage)
		.bind(&org_id)
		.execute(&self.pool)
		.await?;

		Ok(snapshot.id)
	}

	async fn get_snapshot(&self, id: &ContextSnapshotId, org_id: &OrgId) -> Result<ContextSnapshot> {
		let row = sqlx::query_as::<_, SnapshotRow>(
			r#"
			SELECT id, task_id, assembled_at, total_tokens, sections, resource_usage, org_id
			FROM ctx_snapshots
			WHERE id = ? AND org_id = ?
			"#,
		)
		.bind(id.to_string())
		.bind(org_id.to_string())
		.fetch_optional(&self.pool)
		.await?
		.ok_or_else(|| ContextDbError::SnapshotNotFound(id.to_string()))?;

		row.into_context_snapshot()
	}

	async fn list_snapshots_for_task(
		&self,
		org_id: &OrgId,
		task_id: &str,
	) -> Result<Vec<ContextSnapshot>> {
		let rows = sqlx::query_as::<_, SnapshotRow>(
			r#"
			SELECT id, task_id, assembled_at, total_tokens, sections, resource_usage, org_id
			FROM ctx_snapshots
			WHERE org_id = ? AND task_id = ?
			ORDER BY assembled_at DESC
			"#,
		)
		.bind(org_id.to_string())
		.bind(task_id)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(|r| r.into_context_snapshot()).collect()
	}
}

// Row types for SQLite mapping

#[derive(Debug, sqlx::FromRow)]
struct MemoryRow {
	id: String,
	memory_type: String,
	#[allow(dead_code)]
	content_type: String,
	content: String,
	created_at: String,
	last_accessed: String,
	access_count: i64,
	relevance_score: f64,
	tags: String,
	source_type: String,
	source_id: Option<String>,
	org_id: String,
}

impl MemoryRow {
	fn into_memory_item(self) -> Result<MemoryItem> {
		let id: MemoryId = self.id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid memory id: {}", self.id))
		})?;
		let memory_type: MemoryType = self.memory_type.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid memory type: {}", self.memory_type))
		})?;
		let content: MemoryContent = serde_json::from_str(&self.content)?;
		let created_at = DateTime::parse_from_rfc3339(&self.created_at)
			.map_err(|e| ContextDbError::Serialization(e.to_string()))?
			.with_timezone(&Utc);
		let last_accessed = DateTime::parse_from_rfc3339(&self.last_accessed)
			.map_err(|e| ContextDbError::Serialization(e.to_string()))?
			.with_timezone(&Utc);
		let tags: Vec<String> = serde_json::from_str(&self.tags)?;
		let org_id: OrgId = self.org_id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid org id: {}", self.org_id))
		})?;

		let source = match (self.source_type.as_str(), self.source_id) {
			("task", Some(id)) => MemorySource::Task { task_id: id },
			("verification", Some(id)) => MemorySource::Verification { verification_id: id },
			("conversation", Some(id)) => MemorySource::Conversation { conversation_id: id },
			_ => MemorySource::Manual,
		};

		Ok(MemoryItem {
			id,
			memory_type,
			content,
			created_at,
			last_accessed,
			access_count: self.access_count as u32,
			relevance_score: self.relevance_score,
			tags,
			source,
			org_id,
		})
	}
}

#[derive(Debug, sqlx::FromRow)]
struct VerificationRow {
	id: String,
	claim_type: String,
	assertion: String,
	passed: bool,
	output: String,
	verified_at: String,
	duration_ms: i64,
	task_id: Option<String>,
	org_id: String,
}

impl VerificationRow {
	fn into_verification_result(self) -> Result<VerificationResult> {
		use loom_context_core::{Claim, ClaimType};
		use std::time::Duration;

		let id: VerificationId = self.id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid verification id: {}", self.id))
		})?;
		let verified_at = DateTime::parse_from_rfc3339(&self.verified_at)
			.map_err(|e| ContextDbError::Serialization(e.to_string()))?
			.with_timezone(&Utc);
		let org_id: OrgId = self.org_id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid org id: {}", self.org_id))
		})?;

		// Create a minimal claim type since we only stored the type key
		let claim_type = match self.claim_type.as_str() {
			"compiles" => ClaimType::Compiles { files: vec![] },
			"tests_pass" => ClaimType::TestsPass {
				test_pattern: String::new(),
			},
			"file_contains" => ClaimType::FileContains {
				path: std::path::PathBuf::new(),
				pattern: String::new(),
			},
			"api_returns" => ClaimType::ApiReturns {
				endpoint: String::new(),
				expected_status: 200,
			},
			"command_succeeds" => ClaimType::CommandSucceeds {
				command: String::new(),
			},
			"type_checks" => ClaimType::TypeChecks { files: vec![] },
			"lint_passes" => ClaimType::LintPasses { files: vec![] },
			_ => ClaimType::CommandSucceeds {
				command: String::new(),
			},
		};

		let claim = Claim::new(&self.assertion, claim_type);

		Ok(VerificationResult {
			id,
			claim,
			passed: self.passed,
			output: self.output,
			verified_at,
			duration: Duration::from_millis(self.duration_ms as u64),
			task_id: self.task_id,
			org_id,
		})
	}
}

#[derive(Debug, sqlx::FromRow)]
struct DriftSignalRow {
	id: String,
	#[allow(dead_code)]
	signal_type: String,
	details: String,
	detected_at: String,
	corrected_at: Option<String>,
	correction_action: Option<String>,
	org_id: String,
}

impl DriftSignalRow {
	fn into_drift_signal_record(self) -> Result<DriftSignalRecord> {
		let id: DriftSignalId = self.id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid drift signal id: {}", self.id))
		})?;
		let signal: DriftSignal = serde_json::from_str(&self.details)?;
		let detected_at = DateTime::parse_from_rfc3339(&self.detected_at)
			.map_err(|e| ContextDbError::Serialization(e.to_string()))?
			.with_timezone(&Utc);
		let corrected_at = self
			.corrected_at
			.map(|s| DateTime::parse_from_rfc3339(&s))
			.transpose()
			.map_err(|e| ContextDbError::Serialization(e.to_string()))?
			.map(|dt| dt.with_timezone(&Utc));
		let correction_action: Option<CorrectionAction> = self
			.correction_action
			.map(|s| serde_json::from_str(&s))
			.transpose()?;
		let org_id: OrgId = self.org_id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid org id: {}", self.org_id))
		})?;

		Ok(DriftSignalRecord {
			id,
			signal,
			detected_at,
			corrected_at,
			correction_action,
			org_id,
		})
	}
}

#[derive(Debug, sqlx::FromRow)]
struct StrategyPerformanceRow {
	#[allow(dead_code)]
	task_type: String,
	#[allow(dead_code)]
	strategy: String,
	attempts: i64,
	successes: i64,
	avg_duration_ms: Option<i64>,
	avg_tokens_used: Option<i64>,
}

impl StrategyPerformanceRow {
	fn into_performance_stats(self) -> PerformanceStats {
		PerformanceStats {
			attempts: self.attempts as u32,
			successes: self.successes as u32,
			avg_duration_ms: self.avg_duration_ms.map(|v| v as u64),
			avg_tokens_used: self.avg_tokens_used.map(|v| v as u64),
		}
	}
}

fn parse_strategy(name: &str) -> Option<ReasoningStrategy> {
	match name {
		"chain_of_thought" => Some(ReasoningStrategy::ChainOfThought),
		"program_of_thought" => Some(ReasoningStrategy::ProgramOfThought),
		"stepwise_refinement" => Some(ReasoningStrategy::StepwiseRefinement),
		"parallel_exploration" => Some(ReasoningStrategy::ParallelExploration { branches: 2 }),
		"verified_reasoning" => Some(ReasoningStrategy::VerifiedReasoning),
		_ => None,
	}
}

#[derive(Debug, sqlx::FromRow)]
struct SnapshotRow {
	id: String,
	task_id: String,
	assembled_at: String,
	total_tokens: i64,
	sections: String,
	resource_usage: String,
	org_id: String,
}

impl SnapshotRow {
	fn into_context_snapshot(self) -> Result<ContextSnapshot> {
		use loom_context_core::ContextSection;

		let id: ContextSnapshotId = self.id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid snapshot id: {}", self.id))
		})?;
		let assembled_at = DateTime::parse_from_rfc3339(&self.assembled_at)
			.map_err(|e| ContextDbError::Serialization(e.to_string()))?
			.with_timezone(&Utc);
		let sections: Vec<ContextSection> = serde_json::from_str(&self.sections)?;
		let resource_usage: ResourceUsage = serde_json::from_str(&self.resource_usage)?;
		let org_id: OrgId = self.org_id.parse().map_err(|_| {
			ContextDbError::Serialization(format!("invalid org id: {}", self.org_id))
		})?;

		Ok(ContextSnapshot {
			id,
			task_id: self.task_id,
			assembled_at,
			total_tokens: self.total_tokens as u32,
			sections,
			resource_usage,
			org_id,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	// Note: These tests require a test database. In a real implementation,
	// you would use a test database pool helper.

	#[test]
	fn memory_row_conversion() {
		let row = MemoryRow {
			id: MemoryId::new().to_string(),
			memory_type: "short_term".to_string(),
			content_type: "code_pattern".to_string(),
			content: r#"{"type":"code_pattern","pattern":"test","example":"ex","applicability":"app"}"#.to_string(),
			created_at: Utc::now().to_rfc3339(),
			last_accessed: Utc::now().to_rfc3339(),
			access_count: 5,
			relevance_score: 0.8,
			tags: r#"["tag1","tag2"]"#.to_string(),
			source_type: "manual".to_string(),
			source_id: None,
			org_id: OrgId::new().to_string(),
		};

		let item = row.into_memory_item().unwrap();
		assert_eq!(item.memory_type, MemoryType::ShortTerm);
		assert_eq!(item.access_count, 5);
		assert!((item.relevance_score - 0.8).abs() < f64::EPSILON);
	}

	#[test]
	fn verification_row_conversion() {
		let row = VerificationRow {
			id: VerificationId::new().to_string(),
			claim_type: "tests_pass".to_string(),
			assertion: "tests should pass".to_string(),
			passed: true,
			output: "all tests passed".to_string(),
			verified_at: Utc::now().to_rfc3339(),
			duration_ms: 1000,
			task_id: Some("task-1".to_string()),
			org_id: OrgId::new().to_string(),
		};

		let result = row.into_verification_result().unwrap();
		assert!(result.passed);
		assert_eq!(result.output, "all tests passed");
		assert_eq!(result.duration, Duration::from_millis(1000));
	}

	#[test]
	fn drift_signal_row_conversion() {
		let signal = DriftSignal::HighFailureRate { rate: 0.75 };
		let row = DriftSignalRow {
			id: DriftSignalId::new().to_string(),
			signal_type: "high_failure_rate".to_string(),
			details: serde_json::to_string(&signal).unwrap(),
			detected_at: Utc::now().to_rfc3339(),
			corrected_at: None,
			correction_action: None,
			org_id: OrgId::new().to_string(),
		};

		let record = row.into_drift_signal_record().unwrap();
		assert!(matches!(record.signal, DriftSignal::HighFailureRate { .. }));
		assert!(!record.is_corrected());
	}

	#[test]
	fn strategy_performance_row_conversion() {
		let row = StrategyPerformanceRow {
			task_type: "bug_fix".to_string(),
			strategy: "verified_reasoning".to_string(),
			attempts: 10,
			successes: 8,
			avg_duration_ms: Some(5000),
			avg_tokens_used: Some(1000),
		};

		let stats = row.into_performance_stats();
		assert_eq!(stats.attempts, 10);
		assert_eq!(stats.successes, 8);
		assert!((stats.success_rate() - 0.8).abs() < f64::EPSILON);
	}
}
