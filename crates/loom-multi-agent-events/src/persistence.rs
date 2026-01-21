// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event persistence layer for durable event storage and replay.
//!
//! This module provides persistence for task events, enabling:
//! - Event replay for recovery scenarios
//! - Historical event queries
//! - Durable subscriptions

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use loom_multi_agent_core::{PlannerId, ProjectRunId, TaskId, WorkerId};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::instrument;
use uuid::Uuid;

use crate::events::TaskEvent;
use crate::Result;

/// Buffer flush threshold (number of events before flushing to database).
const DEFAULT_FLUSH_THRESHOLD: usize = 100;

/// Metrics for the event persistence layer.
#[derive(Debug, Default)]
pub struct PersistenceMetrics {
	/// Events stored
	pub events_stored: AtomicU64,
	/// Events replayed
	pub events_replayed: AtomicU64,
	/// Flush operations performed
	pub flushes: AtomicU64,
}

impl PersistenceMetrics {
	pub fn stored(&self) -> u64 {
		self.events_stored.load(Ordering::Relaxed)
	}

	pub fn replayed(&self) -> u64 {
		self.events_replayed.load(Ordering::Relaxed)
	}

	pub fn flushes(&self) -> u64 {
		self.flushes.load(Ordering::Relaxed)
	}
}

/// A buffered event waiting to be flushed to the database.
struct BufferedEvent {
	event: TaskEvent,
	run_id: ProjectRunId,
	timestamp: DateTime<Utc>,
}

/// Event persistence layer with buffered writes.
pub struct EventPersistence {
	/// Database pool
	db: SqlitePool,
	/// Write buffer
	buffer: RwLock<Vec<BufferedEvent>>,
	/// Buffer flush threshold
	flush_threshold: usize,
	/// Metrics
	metrics: PersistenceMetrics,
}

impl EventPersistence {
	/// Create a new event persistence layer.
	pub fn new(db: SqlitePool) -> Self {
		Self::with_flush_threshold(db, DEFAULT_FLUSH_THRESHOLD)
	}

	/// Create a new event persistence layer with a custom flush threshold.
	pub fn with_flush_threshold(db: SqlitePool, flush_threshold: usize) -> Self {
		Self {
			db,
			buffer: RwLock::new(Vec::with_capacity(flush_threshold)),
			flush_threshold,
			metrics: PersistenceMetrics::default(),
		}
	}

	/// Get persistence metrics.
	pub fn metrics(&self) -> &PersistenceMetrics {
		&self.metrics
	}

	/// Store an event with optional buffering.
	#[instrument(skip(self, event), fields(event_type = %event.event_type()))]
	pub async fn store(&self, event: &TaskEvent, run_id: &ProjectRunId) -> Result<()> {
		let mut buffer = self.buffer.write().await;
		buffer.push(BufferedEvent {
			event: event.clone(),
			run_id: run_id.clone(),
			timestamp: Utc::now(),
		});

		if buffer.len() >= self.flush_threshold {
			self.flush_buffer(&mut buffer).await?;
		}

		Ok(())
	}

	/// Force flush all buffered events to the database.
	pub async fn flush(&self) -> Result<()> {
		let mut buffer = self.buffer.write().await;
		if !buffer.is_empty() {
			self.flush_buffer(&mut buffer).await?;
		}
		Ok(())
	}

	/// Internal flush implementation.
	async fn flush_buffer(&self, buffer: &mut Vec<BufferedEvent>) -> Result<()> {
		let events: Vec<_> = buffer.drain(..).collect();
		let count = events.len();

		for buffered in events {
			let id = Uuid::now_v7().to_string();
			let event_type = buffered.event.event_type().as_str();
			let task_id = buffered.event.task_id().to_string();
			let planner_id = buffered.event.planner_id().map(|p| p.to_string());
			let worker_id = buffered.event.worker_id().map(|w| w.to_string());
			let payload = serde_json::to_string(&buffered.event)
				.map_err(|e| crate::EventError::Serialization(e.to_string()))?;
			let timestamp = buffered.timestamp.to_rfc3339();

			sqlx::query(
				"INSERT INTO ma_task_events (id, project_run_id, event_type, task_id, planner_id, worker_id, payload, timestamp)
				 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
			)
			.bind(&id)
			.bind(buffered.run_id.to_string())
			.bind(event_type)
			.bind(&task_id)
			.bind(&planner_id)
			.bind(&worker_id)
			.bind(&payload)
			.bind(&timestamp)
			.execute(&self.db)
			.await
			.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;
		}

		self.metrics
			.events_stored
			.fetch_add(count as u64, Ordering::Relaxed);
		self.metrics.flushes.fetch_add(1, Ordering::Relaxed);

		tracing::debug!(count, "Flushed events to database");
		Ok(())
	}

	/// Replay events from a point in time for a project run.
	#[instrument(skip(self))]
	pub async fn replay_from(
		&self,
		run_id: &ProjectRunId,
		since: DateTime<Utc>,
	) -> Result<Vec<TaskEvent>> {
		let rows = sqlx::query_as::<_, (String,)>(
			"SELECT payload FROM ma_task_events
			 WHERE project_run_id = ? AND timestamp >= ?
			 ORDER BY timestamp ASC",
		)
		.bind(run_id.to_string())
		.bind(since.to_rfc3339())
		.fetch_all(&self.db)
		.await
		.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;

		let events: Vec<TaskEvent> = rows
			.iter()
			.filter_map(|(payload,)| serde_json::from_str(payload).ok())
			.collect();

		self.metrics
			.events_replayed
			.fetch_add(events.len() as u64, Ordering::Relaxed);

		tracing::info!(count = events.len(), "Replayed events from database");
		Ok(events)
	}

	/// Replay events for a specific planner.
	#[instrument(skip(self))]
	pub async fn replay_for_planner(
		&self,
		planner_id: &PlannerId,
		since: DateTime<Utc>,
	) -> Result<Vec<TaskEvent>> {
		let rows = sqlx::query_as::<_, (String,)>(
			"SELECT payload FROM ma_task_events
			 WHERE planner_id = ? AND timestamp >= ?
			 ORDER BY timestamp ASC",
		)
		.bind(planner_id.to_string())
		.bind(since.to_rfc3339())
		.fetch_all(&self.db)
		.await
		.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;

		let events: Vec<TaskEvent> = rows
			.iter()
			.filter_map(|(payload,)| serde_json::from_str(payload).ok())
			.collect();

		self.metrics
			.events_replayed
			.fetch_add(events.len() as u64, Ordering::Relaxed);

		tracing::info!(count = events.len(), %planner_id, "Replayed events for planner");
		Ok(events)
	}

	/// Query event history with filters.
	#[instrument(skip(self))]
	pub async fn query_history(
		&self,
		run_id: &ProjectRunId,
		query: EventHistoryQuery,
	) -> Result<EventHistoryResult> {
		let mut sql = String::from(
			"SELECT id, event_type, task_id, planner_id, worker_id, payload, timestamp
			 FROM ma_task_events WHERE project_run_id = ?",
		);
		let mut count_sql =
			String::from("SELECT COUNT(*) as total FROM ma_task_events WHERE project_run_id = ?");

		if let Some(ref since) = query.since {
			let clause = format!(" AND timestamp >= '{}'", since.to_rfc3339());
			sql.push_str(&clause);
			count_sql.push_str(&clause);
		}

		if let Some(ref until) = query.until {
			let clause = format!(" AND timestamp <= '{}'", until.to_rfc3339());
			sql.push_str(&clause);
			count_sql.push_str(&clause);
		}

		if let Some(ref event_types) = query.event_types {
			let types: Vec<_> = event_types.iter().map(|t| format!("'{}'", t)).collect();
			let clause = format!(" AND event_type IN ({})", types.join(","));
			sql.push_str(&clause);
			count_sql.push_str(&clause);
		}

		if let Some(ref planner_id) = query.planner_id {
			let clause = format!(" AND planner_id = '{}'", planner_id);
			sql.push_str(&clause);
			count_sql.push_str(&clause);
		}

		if let Some(ref task_id) = query.task_id {
			let clause = format!(" AND task_id = '{}'", task_id);
			sql.push_str(&clause);
			count_sql.push_str(&clause);
		}

		sql.push_str(" ORDER BY timestamp ASC");

		let total: (i64,) = sqlx::query_as(&count_sql)
			.bind(run_id.to_string())
			.fetch_one(&self.db)
			.await
			.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;

		let limit = query.limit.unwrap_or(100) as i64;
		let offset = query.offset.unwrap_or(0) as i64;
		sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

		let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, String, String)>(
			&sql,
		)
		.bind(run_id.to_string())
		.fetch_all(&self.db)
		.await
		.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;

		let events: Vec<StoredEvent> = rows
			.into_iter()
			.filter_map(|(id, event_type, task_id, planner_id, worker_id, payload, timestamp)| {
				let event: TaskEvent = serde_json::from_str(&payload).ok()?;
				let ts = DateTime::parse_from_rfc3339(&timestamp).ok()?.with_timezone(&Utc);
				Some(StoredEvent {
					id,
					event_type,
					task_id,
					planner_id,
					worker_id,
					event,
					timestamp: ts,
				})
			})
			.collect();

		let has_more = (offset + events.len() as i64) < total.0;

		Ok(EventHistoryResult {
			events,
			total: total.0 as u64,
			has_more,
		})
	}

	/// Mark events as processed.
	pub async fn mark_processed(&self, event_ids: &[String]) -> Result<()> {
		if event_ids.is_empty() {
			return Ok(());
		}

		let placeholders: Vec<_> = event_ids.iter().map(|_| "?").collect();
		let sql = format!(
			"UPDATE ma_task_events SET processed = 1 WHERE id IN ({})",
			placeholders.join(",")
		);

		let mut query = sqlx::query(&sql);
		for id in event_ids {
			query = query.bind(id);
		}

		query
			.execute(&self.db)
			.await
			.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;

		Ok(())
	}

	/// Get unprocessed events for a project run.
	pub async fn get_unprocessed(&self, run_id: &ProjectRunId) -> Result<Vec<StoredEvent>> {
		let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, String, String)>(
			"SELECT id, event_type, task_id, planner_id, worker_id, payload, timestamp
			 FROM ma_task_events
			 WHERE project_run_id = ? AND processed = 0
			 ORDER BY timestamp ASC",
		)
		.bind(run_id.to_string())
		.fetch_all(&self.db)
		.await
		.map_err(|e| crate::EventError::SendFailed(e.to_string()))?;

		let events: Vec<StoredEvent> = rows
			.into_iter()
			.filter_map(|(id, event_type, task_id, planner_id, worker_id, payload, timestamp)| {
				let event: TaskEvent = serde_json::from_str(&payload).ok()?;
				let ts = DateTime::parse_from_rfc3339(&timestamp).ok()?.with_timezone(&Utc);
				Some(StoredEvent {
					id,
					event_type,
					task_id,
					planner_id,
					worker_id,
					event,
					timestamp: ts,
				})
			})
			.collect();

		Ok(events)
	}
}

/// A stored event with metadata.
#[derive(Debug, Clone)]
pub struct StoredEvent {
	/// Event ID
	pub id: String,
	/// Event type string
	pub event_type: String,
	/// Task ID if applicable
	pub task_id: Option<String>,
	/// Planner ID if applicable
	pub planner_id: Option<String>,
	/// Worker ID if applicable
	pub worker_id: Option<String>,
	/// The actual event
	pub event: TaskEvent,
	/// Timestamp when stored
	pub timestamp: DateTime<Utc>,
}

/// Query parameters for event history.
#[derive(Debug, Clone, Default)]
pub struct EventHistoryQuery {
	/// Filter by events after this time
	pub since: Option<DateTime<Utc>>,
	/// Filter by events before this time
	pub until: Option<DateTime<Utc>>,
	/// Filter by event types
	pub event_types: Option<Vec<String>>,
	/// Filter by planner ID
	pub planner_id: Option<String>,
	/// Filter by task ID
	pub task_id: Option<String>,
	/// Maximum number of events to return
	pub limit: Option<u32>,
	/// Offset for pagination
	pub offset: Option<u32>,
}

/// Result of an event history query.
#[derive(Debug)]
pub struct EventHistoryResult {
	/// Matching events
	pub events: Vec<StoredEvent>,
	/// Total count of matching events
	pub total: u64,
	/// Whether there are more events
	pub has_more: bool,
}
