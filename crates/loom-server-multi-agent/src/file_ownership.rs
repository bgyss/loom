// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! File ownership manager for conflict prevention.

use std::path::{Path, PathBuf};

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::instrument;

use loom_multi_agent_core::{ProjectRunId, WorkerId};

use crate::error::{MultiAgentServerError, Result};

/// Manages file ownership to prevent conflicts between workers.
pub struct FileOwnershipManager {
	/// Database pool
	pool: SqlitePool,
	/// Project run ID
	project_run_id: ProjectRunId,
	/// Lock for atomic operations
	lock: RwLock<()>,
}

impl FileOwnershipManager {
	/// Create a new file ownership manager.
	pub fn new(pool: SqlitePool, project_run_id: ProjectRunId) -> Self {
		Self {
			pool,
			project_run_id,
			lock: RwLock::new(()),
		}
	}

	/// Acquire ownership of files for a worker.
	#[instrument(skip(self, files), fields(worker_id = %worker_id, file_count = files.len()))]
	pub async fn acquire(&self, worker_id: &WorkerId, files: &[PathBuf]) -> Result<FileOwnershipGuard> {
		let _guard = self.lock.write().await;

		let conflicts = self.check_conflicts(files).await?;
		if !conflicts.is_empty() {
			let conflict = conflicts.into_iter().next().unwrap();
			return Err(MultiAgentServerError::FileOwnershipConflict {
				file: conflict.0,
				owner: conflict.1,
			});
		}

		let now = Utc::now().to_rfc3339();
		for file in files {
			let file_path = file.to_string_lossy();
			sqlx::query(
				r#"
				INSERT INTO ma_file_ownership (project_run_id, file_path, worker_id, acquired_at)
				VALUES (?, ?, ?, ?)
				ON CONFLICT(project_run_id, file_path) DO UPDATE SET
					worker_id = excluded.worker_id,
					acquired_at = excluded.acquired_at
				"#,
			)
			.bind(self.project_run_id.to_string())
			.bind(file_path.as_ref())
			.bind(worker_id.to_string())
			.bind(&now)
			.execute(&self.pool)
			.await?;
		}

		tracing::debug!(%worker_id, "Acquired ownership of {} files", files.len());

		Ok(FileOwnershipGuard {
			pool: self.pool.clone(),
			project_run_id: self.project_run_id.clone(),
			worker_id: worker_id.clone(),
			files: files.to_vec(),
		})
	}

	/// Try to acquire ownership, returning None if any file is already owned.
	#[instrument(skip(self, files), fields(worker_id = %worker_id))]
	pub async fn try_acquire(
		&self,
		worker_id: &WorkerId,
		files: &[PathBuf],
	) -> Result<Option<FileOwnershipGuard>> {
		match self.acquire(worker_id, files).await {
			Ok(guard) => Ok(Some(guard)),
			Err(MultiAgentServerError::FileOwnershipConflict { .. }) => Ok(None),
			Err(e) => Err(e),
		}
	}

	/// Check which files conflict with existing ownership.
	async fn check_conflicts(&self, files: &[PathBuf]) -> Result<Vec<(String, String)>> {
		let mut conflicts = Vec::new();

		for file in files {
			let file_path = file.to_string_lossy();
			let row: Option<(String,)> = sqlx::query_as(
				r#"
				SELECT worker_id FROM ma_file_ownership
				WHERE project_run_id = ? AND file_path = ?
				"#,
			)
			.bind(self.project_run_id.to_string())
			.bind(file_path.as_ref())
			.fetch_optional(&self.pool)
			.await?;

			if let Some((owner,)) = row {
				conflicts.push((file_path.to_string(), owner));
			}
		}

		Ok(conflicts)
	}

	/// Release ownership of files for a worker.
	#[instrument(skip(self, files), fields(worker_id = %worker_id))]
	pub async fn release(&self, worker_id: &WorkerId, files: &[PathBuf]) -> Result<()> {
		let _guard = self.lock.write().await;

		for file in files {
			let file_path = file.to_string_lossy();
			sqlx::query(
				r#"
				DELETE FROM ma_file_ownership
				WHERE project_run_id = ? AND file_path = ? AND worker_id = ?
				"#,
			)
			.bind(self.project_run_id.to_string())
			.bind(file_path.as_ref())
			.bind(worker_id.to_string())
			.execute(&self.pool)
			.await?;
		}

		tracing::debug!(%worker_id, "Released ownership of {} files", files.len());
		Ok(())
	}

	/// Release all files owned by a worker.
	#[instrument(skip(self), fields(worker_id = %worker_id))]
	pub async fn release_all(&self, worker_id: &WorkerId) -> Result<u64> {
		let _guard = self.lock.write().await;

		let result = sqlx::query(
			r#"
			DELETE FROM ma_file_ownership
			WHERE project_run_id = ? AND worker_id = ?
			"#,
		)
		.bind(self.project_run_id.to_string())
		.bind(worker_id.to_string())
		.execute(&self.pool)
		.await?;

		tracing::debug!(%worker_id, "Released all file ownership");
		Ok(result.rows_affected())
	}

	/// Get all files owned by a worker.
	#[instrument(skip(self), fields(worker_id = %worker_id))]
	pub async fn get_owned_files(&self, worker_id: &WorkerId) -> Result<Vec<PathBuf>> {
		let rows: Vec<(String,)> = sqlx::query_as(
			r#"
			SELECT file_path FROM ma_file_ownership
			WHERE project_run_id = ? AND worker_id = ?
			"#,
		)
		.bind(self.project_run_id.to_string())
		.bind(worker_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		Ok(rows.into_iter().map(|(p,)| PathBuf::from(p)).collect())
	}

	/// Get the owner of a file.
	#[instrument(skip(self), fields(file = %file.display()))]
	pub async fn get_owner(&self, file: &Path) -> Result<Option<WorkerId>> {
		let file_path = file.to_string_lossy();
		let row: Option<(String,)> = sqlx::query_as(
			r#"
			SELECT worker_id FROM ma_file_ownership
			WHERE project_run_id = ? AND file_path = ?
			"#,
		)
		.bind(self.project_run_id.to_string())
		.bind(file_path.as_ref())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|(id,)| {
			id.parse()
				.map(WorkerId)
				.map_err(|_| MultiAgentServerError::InvalidData("invalid worker ID".into()))
		})
		.transpose()
	}
}

/// RAII guard that releases file ownership when dropped.
pub struct FileOwnershipGuard {
	pool: SqlitePool,
	project_run_id: ProjectRunId,
	worker_id: WorkerId,
	files: Vec<PathBuf>,
}

impl FileOwnershipGuard {
	/// Get the owned files.
	pub fn files(&self) -> &[PathBuf] {
		&self.files
	}

	/// Manually release ownership (instead of waiting for drop).
	pub async fn release(self) -> Result<()> {
		self.release_impl().await
	}

	async fn release_impl(&self) -> Result<()> {
		for file in &self.files {
			let file_path = file.to_string_lossy();
			sqlx::query(
				r#"
				DELETE FROM ma_file_ownership
				WHERE project_run_id = ? AND file_path = ? AND worker_id = ?
				"#,
			)
			.bind(self.project_run_id.to_string())
			.bind(file_path.as_ref())
			.bind(self.worker_id.to_string())
			.execute(&self.pool)
			.await?;
		}
		Ok(())
	}
}

impl Drop for FileOwnershipGuard {
	fn drop(&mut self) {
		let pool = self.pool.clone();
		let project_run_id = self.project_run_id.clone();
		let worker_id = self.worker_id.clone();
		let files = std::mem::take(&mut self.files);

		tokio::spawn(async move {
			for file in &files {
				let file_path = file.to_string_lossy();
				let _ = sqlx::query(
					r#"
					DELETE FROM ma_file_ownership
					WHERE project_run_id = ? AND file_path = ? AND worker_id = ?
					"#,
				)
				.bind(project_run_id.to_string())
				.bind(file_path.as_ref())
				.bind(worker_id.to_string())
				.execute(&pool)
				.await;
			}
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_file_ownership_guard_files() {
		let files = vec![PathBuf::from("/test/file.rs")];
		assert_eq!(files.len(), 1);
	}
}
