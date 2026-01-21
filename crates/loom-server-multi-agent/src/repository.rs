// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Repository layer for multi-agent database operations.

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tracing::instrument;

use loom_multi_agent_core::{
	CheckpointId, Domain, DomainId, PlannerId, ProjectConfig, ProjectMetrics, ProjectPhase,
	ProjectRunId, ProjectState, ProjectStatus, Task, TaskCompletion, TaskId, TaskOutput,
	TaskSpecification, TaskStatus, TaskType, WorkerId,
};

use crate::error::{MultiAgentServerError, Result};

/// Repository trait for multi-agent operations.
#[async_trait]
pub trait MultiAgentRepository: Send + Sync {
	// Project run operations
	async fn create_project_run(&self, state: &ProjectState) -> Result<()>;
	async fn get_project_run(&self, id: &ProjectRunId) -> Result<Option<ProjectState>>;
	async fn update_project_run(&self, state: &ProjectState) -> Result<()>;
	async fn list_project_runs(&self, org_id: &str, limit: u32, offset: u32)
		-> Result<Vec<ProjectState>>;
	async fn delete_project_run(&self, id: &ProjectRunId) -> Result<()>;

	// Domain operations
	async fn create_domain(&self, project_run_id: &ProjectRunId, domain: &Domain) -> Result<()>;
	async fn get_domain(&self, id: &DomainId) -> Result<Option<Domain>>;
	async fn list_domains(&self, project_run_id: &ProjectRunId) -> Result<Vec<Domain>>;

	// Planner operations
	async fn create_planner(
		&self,
		id: &PlannerId,
		domain_id: &DomainId,
		parent_id: Option<&PlannerId>,
	) -> Result<()>;
	async fn update_planner_status(&self, id: &PlannerId, status: &str) -> Result<()>;
	async fn get_planner_domain(&self, id: &PlannerId) -> Result<Option<DomainId>>;

	// Task operations
	async fn create_task(&self, task: &Task) -> Result<()>;
	async fn get_task(&self, id: &TaskId) -> Result<Option<Task>>;
	async fn update_task(&self, task: &Task) -> Result<()>;
	async fn update_task_status(&self, id: &TaskId, status: &TaskStatus) -> Result<()>;
	async fn list_tasks_by_planner(&self, planner_id: &PlannerId) -> Result<Vec<Task>>;
	async fn list_tasks_by_status(&self, project_run_id: &ProjectRunId, status: &str)
		-> Result<Vec<Task>>;
	async fn get_ready_tasks(&self, project_run_id: &ProjectRunId, limit: u32) -> Result<Vec<Task>>;
	async fn add_task_dependency(&self, task_id: &TaskId, depends_on: &TaskId) -> Result<()>;
	async fn remove_task_dependency(&self, task_id: &TaskId, depends_on: &TaskId) -> Result<()>;
	async fn get_task_dependencies(&self, task_id: &TaskId) -> Result<Vec<TaskId>>;
	async fn get_dependent_tasks(&self, task_id: &TaskId) -> Result<Vec<TaskId>>;

	// Worker operations
	async fn create_worker(&self, id: &WorkerId, project_run_id: &ProjectRunId) -> Result<()>;
	async fn get_worker_task(&self, id: &WorkerId) -> Result<Option<TaskId>>;
	async fn assign_task_to_worker(&self, worker_id: &WorkerId, task_id: &TaskId) -> Result<()>;
	async fn clear_worker_task(&self, id: &WorkerId) -> Result<()>;
	async fn update_worker_status(&self, id: &WorkerId, status: &str) -> Result<()>;
	async fn list_workers(&self, project_run_id: &ProjectRunId) -> Result<Vec<WorkerId>>;

	// Completion operations
	async fn create_completion(&self, completion: &TaskCompletion) -> Result<()>;
	async fn get_completion(&self, task_id: &TaskId) -> Result<Option<TaskCompletion>>;

	// Checkpoint operations
	async fn create_checkpoint(
		&self,
		id: &CheckpointId,
		project_run_id: &ProjectRunId,
		snapshot: &str,
		reason: Option<&str>,
	) -> Result<()>;
	async fn get_checkpoint(&self, id: &CheckpointId) -> Result<Option<String>>;
	async fn list_checkpoints(&self, project_run_id: &ProjectRunId) -> Result<Vec<CheckpointId>>;
}

/// SQLite implementation of the multi-agent repository.
#[derive(Clone)]
pub struct SqliteMultiAgentRepository {
	pool: SqlitePool,
}

impl SqliteMultiAgentRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}
}

// Database row structs
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct ProjectRunRow {
	id: String,
	org_id: String,
	project_path: String,
	goal: String,
	phase: String,
	status: String,
	config: Option<String>,
	metrics: Option<String>,
	started_at: String,
	completed_at: Option<String>,
	created_by: String,
}

impl TryFrom<ProjectRunRow> for ProjectState {
	type Error = MultiAgentServerError;

	fn try_from(row: ProjectRunRow) -> Result<Self> {
		let id = ProjectRunId(
			row.id
				.parse()
				.map_err(|_| MultiAgentServerError::InvalidData("invalid project run ID".into()))?,
		);

		let phase: ProjectPhase = row
			.phase
			.parse()
			.map_err(|e| MultiAgentServerError::InvalidData(format!("invalid phase: {e}")))?;

		let status: ProjectStatus = row
			.status
			.parse()
			.map_err(|e| MultiAgentServerError::InvalidData(format!("invalid status: {e}")))?;

		let metrics: ProjectMetrics = row
			.metrics
			.map(|m| serde_json::from_str(&m))
			.transpose()?
			.unwrap_or_default();

		let started_at = DateTime::parse_from_rfc3339(&row.started_at)
			.map_err(|e| MultiAgentServerError::InvalidData(format!("invalid started_at: {e}")))?
			.with_timezone(&Utc);

		let completed_at = row
			.completed_at
			.map(|s| {
				DateTime::parse_from_rfc3339(&s)
					.map(|dt| dt.with_timezone(&Utc))
					.map_err(|e| {
						MultiAgentServerError::InvalidData(format!("invalid completed_at: {e}"))
					})
			})
			.transpose()?;

		Ok(ProjectState {
			id,
			org_id: row.org_id,
			project_path: row.project_path,
			goal: row.goal,
			phase,
			status,
			domains: Default::default(),
			metrics,
			checkpoints: Vec::new(),
			started_at,
			completed_at,
			created_by: row.created_by,
		})
	}
}

#[derive(sqlx::FromRow)]
struct DomainRow {
	id: String,
	name: String,
	file_patterns: String,
	created_at: String,
}

impl TryFrom<DomainRow> for Domain {
	type Error = MultiAgentServerError;

	fn try_from(row: DomainRow) -> Result<Self> {
		let id = DomainId(
			row.id
				.parse()
				.map_err(|_| MultiAgentServerError::InvalidData("invalid domain ID".into()))?,
		);

		let file_patterns: Vec<String> = serde_json::from_str(&row.file_patterns)?;

		let created_at = DateTime::parse_from_rfc3339(&row.created_at)
			.map_err(|e| MultiAgentServerError::InvalidData(format!("invalid created_at: {e}")))?
			.with_timezone(&Utc);

		Ok(Domain {
			id,
			name: row.name,
			file_patterns,
			dependencies: Vec::new(),
			created_at,
		})
	}
}

#[derive(sqlx::FromRow)]
struct TaskRow {
	id: String,
	parent_id: Option<String>,
	planner_id: String,
	priority: i32,
	task_type: String,
	description: String,
	specification: String,
	affected_files: String,
	status: String,
	iteration: i32,
	created_at: String,
	updated_at: String,
}

impl TryFrom<TaskRow> for Task {
	type Error = MultiAgentServerError;

	fn try_from(row: TaskRow) -> Result<Self> {
		let id = TaskId(
			row.id
				.parse()
				.map_err(|_| MultiAgentServerError::InvalidData("invalid task ID".into()))?,
		);

		let parent_id = row
			.parent_id
			.map(|s| {
				s.parse()
					.map(TaskId)
					.map_err(|_| MultiAgentServerError::InvalidData("invalid parent ID".into()))
			})
			.transpose()?;

		let created_by = PlannerId(
			row.planner_id
				.parse()
				.map_err(|_| MultiAgentServerError::InvalidData("invalid planner ID".into()))?,
		);

		let task_type: TaskType = serde_json::from_str(&row.task_type)?;
		let specification: TaskSpecification = serde_json::from_str(&row.specification)?;
		let affected_files: Vec<PathBuf> = serde_json::from_str(&row.affected_files)?;
		let status: TaskStatus = serde_json::from_str(&row.status)?;

		let created_at = DateTime::parse_from_rfc3339(&row.created_at)
			.map_err(|e| MultiAgentServerError::InvalidData(format!("invalid created_at: {e}")))?
			.with_timezone(&Utc);

		let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
			.map_err(|e| MultiAgentServerError::InvalidData(format!("invalid updated_at: {e}")))?
			.with_timezone(&Utc);

		Ok(Task {
			id,
			parent_id,
			created_by,
			priority: row.priority,
			task_type,
			description: row.description,
			specification,
			affected_files,
			dependencies: Vec::new(),
			status,
			deadline: None,
			context_snapshot: Default::default(),
			iteration: row.iteration as u32,
			created_at,
			updated_at,
		})
	}
}

#[async_trait]
impl MultiAgentRepository for SqliteMultiAgentRepository {
	#[instrument(skip(self, state), fields(project_run_id = %state.id))]
	async fn create_project_run(&self, state: &ProjectState) -> Result<()> {
		let config_json = serde_json::to_string(&ProjectConfig::default())?;
		let metrics_json = serde_json::to_string(&state.metrics)?;

		sqlx::query(
			r#"
			INSERT INTO ma_project_runs (
				id, org_id, project_path, goal, phase, status,
				config, metrics, started_at, completed_at, created_by
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(state.id.to_string())
		.bind(&state.org_id)
		.bind(&state.project_path)
		.bind(&state.goal)
		.bind(state.phase.to_string())
		.bind(state.status.to_string())
		.bind(&config_json)
		.bind(&metrics_json)
		.bind(state.started_at.to_rfc3339())
		.bind(state.completed_at.map(|dt| dt.to_rfc3339()))
		.bind(&state.created_by)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(project_run_id = %id))]
	async fn get_project_run(&self, id: &ProjectRunId) -> Result<Option<ProjectState>> {
		let row = sqlx::query_as::<_, ProjectRunRow>(
			r#"
			SELECT id, org_id, project_path, goal, phase, status,
				   config, metrics, started_at, completed_at, created_by
			FROM ma_project_runs
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self, state), fields(project_run_id = %state.id))]
	async fn update_project_run(&self, state: &ProjectState) -> Result<()> {
		let metrics_json = serde_json::to_string(&state.metrics)?;

		sqlx::query(
			r#"
			UPDATE ma_project_runs SET
				phase = ?,
				status = ?,
				metrics = ?,
				completed_at = ?
			WHERE id = ?
			"#,
		)
		.bind(state.phase.to_string())
		.bind(state.status.to_string())
		.bind(&metrics_json)
		.bind(state.completed_at.map(|dt| dt.to_rfc3339()))
		.bind(state.id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(org_id = %org_id))]
	async fn list_project_runs(
		&self,
		org_id: &str,
		limit: u32,
		offset: u32,
	) -> Result<Vec<ProjectState>> {
		let rows = sqlx::query_as::<_, ProjectRunRow>(
			r#"
			SELECT id, org_id, project_path, goal, phase, status,
				   config, metrics, started_at, completed_at, created_by
			FROM ma_project_runs
			WHERE org_id = ?
			ORDER BY started_at DESC
			LIMIT ? OFFSET ?
			"#,
		)
		.bind(org_id)
		.bind(limit as i32)
		.bind(offset as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(project_run_id = %id))]
	async fn delete_project_run(&self, id: &ProjectRunId) -> Result<()> {
		sqlx::query("DELETE FROM ma_project_runs WHERE id = ?")
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		Ok(())
	}

	#[instrument(skip(self, domain), fields(domain_id = %domain.id))]
	async fn create_domain(&self, project_run_id: &ProjectRunId, domain: &Domain) -> Result<()> {
		let file_patterns_json = serde_json::to_string(&domain.file_patterns)?;

		sqlx::query(
			r#"
			INSERT INTO ma_domains (id, project_run_id, name, file_patterns, created_at)
			VALUES (?, ?, ?, ?, ?)
			"#,
		)
		.bind(domain.id.to_string())
		.bind(project_run_id.to_string())
		.bind(&domain.name)
		.bind(&file_patterns_json)
		.bind(domain.created_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(domain_id = %id))]
	async fn get_domain(&self, id: &DomainId) -> Result<Option<Domain>> {
		let row = sqlx::query_as::<_, DomainRow>(
			r#"
			SELECT id, name, file_patterns, created_at
			FROM ma_domains
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(TryInto::try_into).transpose()
	}

	#[instrument(skip(self), fields(project_run_id = %project_run_id))]
	async fn list_domains(&self, project_run_id: &ProjectRunId) -> Result<Vec<Domain>> {
		let rows = sqlx::query_as::<_, DomainRow>(
			r#"
			SELECT id, name, file_patterns, created_at
			FROM ma_domains
			WHERE project_run_id = ?
			"#,
		)
		.bind(project_run_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(planner_id = %id))]
	async fn create_planner(
		&self,
		id: &PlannerId,
		domain_id: &DomainId,
		parent_id: Option<&PlannerId>,
	) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			INSERT INTO ma_planners (id, domain_id, parent_planner_id, status, created_at, last_active_at)
			VALUES (?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(id.to_string())
		.bind(domain_id.to_string())
		.bind(parent_id.map(|p| p.to_string()))
		.bind("active")
		.bind(&now)
		.bind(&now)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(planner_id = %id))]
	async fn update_planner_status(&self, id: &PlannerId, status: &str) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE ma_planners SET status = ?, last_active_at = ?
			WHERE id = ?
			"#,
		)
		.bind(status)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(planner_id = %id))]
	async fn get_planner_domain(&self, id: &PlannerId) -> Result<Option<DomainId>> {
		let row: Option<(String,)> = sqlx::query_as(
			r#"
			SELECT domain_id FROM ma_planners WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|(domain_id,)| {
			domain_id
				.parse()
				.map(DomainId)
				.map_err(|_| MultiAgentServerError::InvalidData("invalid domain ID".into()))
		})
		.transpose()
	}

	#[instrument(skip(self, task), fields(task_id = %task.id))]
	async fn create_task(&self, task: &Task) -> Result<()> {
		let task_type_json = serde_json::to_string(&task.task_type)?;
		let spec_json = serde_json::to_string(&task.specification)?;
		let files_json = serde_json::to_string(&task.affected_files)?;
		let status_json = serde_json::to_string(&task.status)?;

		sqlx::query(
			r#"
			INSERT INTO ma_tasks (
				id, parent_id, planner_id, priority, task_type,
				description, specification, affected_files, status,
				iteration, created_at, updated_at
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(task.id.to_string())
		.bind(task.parent_id.as_ref().map(|id| id.to_string()))
		.bind(task.created_by.to_string())
		.bind(task.priority)
		.bind(&task_type_json)
		.bind(&task.description)
		.bind(&spec_json)
		.bind(&files_json)
		.bind(&status_json)
		.bind(task.iteration as i32)
		.bind(task.created_at.to_rfc3339())
		.bind(task.updated_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(task_id = %id))]
	async fn get_task(&self, id: &TaskId) -> Result<Option<Task>> {
		let row = sqlx::query_as::<_, TaskRow>(
			r#"
			SELECT id, parent_id, planner_id, priority, task_type,
				   description, specification, affected_files, status,
				   iteration, created_at, updated_at
			FROM ma_tasks
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		let mut task: Option<Task> = row.map(TryInto::try_into).transpose()?;

		if let Some(ref mut t) = task {
			t.dependencies = self.get_task_dependencies(&t.id).await?;
		}

		Ok(task)
	}

	#[instrument(skip(self, task), fields(task_id = %task.id))]
	async fn update_task(&self, task: &Task) -> Result<()> {
		let status_json = serde_json::to_string(&task.status)?;
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE ma_tasks SET
				status = ?,
				iteration = ?,
				updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&status_json)
		.bind(task.iteration as i32)
		.bind(&now)
		.bind(task.id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(task_id = %id))]
	async fn update_task_status(&self, id: &TaskId, status: &TaskStatus) -> Result<()> {
		let status_json = serde_json::to_string(status)?;
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE ma_tasks SET status = ?, updated_at = ?
			WHERE id = ?
			"#,
		)
		.bind(&status_json)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(planner_id = %planner_id))]
	async fn list_tasks_by_planner(&self, planner_id: &PlannerId) -> Result<Vec<Task>> {
		let rows = sqlx::query_as::<_, TaskRow>(
			r#"
			SELECT id, parent_id, planner_id, priority, task_type,
				   description, specification, affected_files, status,
				   iteration, created_at, updated_at
			FROM ma_tasks
			WHERE planner_id = ?
			ORDER BY priority DESC, created_at ASC
			"#,
		)
		.bind(planner_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(project_run_id = %project_run_id, status = %status))]
	async fn list_tasks_by_status(
		&self,
		project_run_id: &ProjectRunId,
		status: &str,
	) -> Result<Vec<Task>> {
		let rows = sqlx::query_as::<_, TaskRow>(
			r#"
			SELECT t.id, t.parent_id, t.planner_id, t.priority, t.task_type,
				   t.description, t.specification, t.affected_files, t.status,
				   t.iteration, t.created_at, t.updated_at
			FROM ma_tasks t
			JOIN ma_planners p ON t.planner_id = p.id
			JOIN ma_domains d ON p.domain_id = d.id
			WHERE d.project_run_id = ? AND t.status LIKE ?
			ORDER BY t.priority DESC, t.created_at ASC
			"#,
		)
		.bind(project_run_id.to_string())
		.bind(format!("%\"status\":\"{}\"%", status))
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(project_run_id = %project_run_id))]
	async fn get_ready_tasks(&self, project_run_id: &ProjectRunId, limit: u32) -> Result<Vec<Task>> {
		let rows = sqlx::query_as::<_, TaskRow>(
			r#"
			SELECT t.id, t.parent_id, t.planner_id, t.priority, t.task_type,
				   t.description, t.specification, t.affected_files, t.status,
				   t.iteration, t.created_at, t.updated_at
			FROM ma_tasks t
			JOIN ma_planners p ON t.planner_id = p.id
			JOIN ma_domains d ON p.domain_id = d.id
			WHERE d.project_run_id = ?
			  AND t.status LIKE '%"status":"pending"%'
			  AND NOT EXISTS (
				  SELECT 1 FROM ma_task_dependencies dep
				  JOIN ma_tasks dep_task ON dep.depends_on_task_id = dep_task.id
				  WHERE dep.task_id = t.id
				  AND dep_task.status NOT LIKE '%"status":"completed"%'
			  )
			ORDER BY t.priority DESC, t.created_at ASC
			LIMIT ?
			"#,
		)
		.bind(project_run_id.to_string())
		.bind(limit as i32)
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter().map(TryInto::try_into).collect()
	}

	#[instrument(skip(self), fields(task_id = %task_id, depends_on = %depends_on))]
	async fn add_task_dependency(&self, task_id: &TaskId, depends_on: &TaskId) -> Result<()> {
		sqlx::query(
			r#"
			INSERT OR IGNORE INTO ma_task_dependencies (task_id, depends_on_task_id)
			VALUES (?, ?)
			"#,
		)
		.bind(task_id.to_string())
		.bind(depends_on.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(task_id = %task_id, depends_on = %depends_on))]
	async fn remove_task_dependency(&self, task_id: &TaskId, depends_on: &TaskId) -> Result<()> {
		sqlx::query(
			r#"
			DELETE FROM ma_task_dependencies
			WHERE task_id = ? AND depends_on_task_id = ?
			"#,
		)
		.bind(task_id.to_string())
		.bind(depends_on.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(task_id = %task_id))]
	async fn get_task_dependencies(&self, task_id: &TaskId) -> Result<Vec<TaskId>> {
		let rows: Vec<(String,)> = sqlx::query_as(
			r#"
			SELECT depends_on_task_id FROM ma_task_dependencies WHERE task_id = ?
			"#,
		)
		.bind(task_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter()
			.map(|(id,)| {
				id.parse()
					.map(TaskId)
					.map_err(|_| MultiAgentServerError::InvalidData("invalid task ID".into()))
			})
			.collect()
	}

	#[instrument(skip(self), fields(task_id = %task_id))]
	async fn get_dependent_tasks(&self, task_id: &TaskId) -> Result<Vec<TaskId>> {
		let rows: Vec<(String,)> = sqlx::query_as(
			r#"
			SELECT task_id FROM ma_task_dependencies WHERE depends_on_task_id = ?
			"#,
		)
		.bind(task_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter()
			.map(|(id,)| {
				id.parse()
					.map(TaskId)
					.map_err(|_| MultiAgentServerError::InvalidData("invalid task ID".into()))
			})
			.collect()
	}

	#[instrument(skip(self), fields(worker_id = %id))]
	async fn create_worker(&self, id: &WorkerId, project_run_id: &ProjectRunId) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			INSERT INTO ma_workers (id, project_run_id, current_task_id, status, created_at, last_active_at)
			VALUES (?, ?, NULL, ?, ?, ?)
			"#,
		)
		.bind(id.to_string())
		.bind(project_run_id.to_string())
		.bind("idle")
		.bind(&now)
		.bind(&now)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(worker_id = %id))]
	async fn get_worker_task(&self, id: &WorkerId) -> Result<Option<TaskId>> {
		let row: Option<(Option<String>,)> = sqlx::query_as(
			r#"
			SELECT current_task_id FROM ma_workers WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.and_then(|(task_id,)| task_id)
			.map(|id| {
				id.parse()
					.map(TaskId)
					.map_err(|_| MultiAgentServerError::InvalidData("invalid task ID".into()))
			})
			.transpose()
	}

	#[instrument(skip(self), fields(worker_id = %worker_id, task_id = %task_id))]
	async fn assign_task_to_worker(&self, worker_id: &WorkerId, task_id: &TaskId) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE ma_workers SET current_task_id = ?, status = ?, last_active_at = ?
			WHERE id = ?
			"#,
		)
		.bind(task_id.to_string())
		.bind("busy")
		.bind(&now)
		.bind(worker_id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(worker_id = %id))]
	async fn clear_worker_task(&self, id: &WorkerId) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE ma_workers SET current_task_id = NULL, status = ?, last_active_at = ?
			WHERE id = ?
			"#,
		)
		.bind("idle")
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(worker_id = %id))]
	async fn update_worker_status(&self, id: &WorkerId, status: &str) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			UPDATE ma_workers SET status = ?, last_active_at = ?
			WHERE id = ?
			"#,
		)
		.bind(status)
		.bind(&now)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(project_run_id = %project_run_id))]
	async fn list_workers(&self, project_run_id: &ProjectRunId) -> Result<Vec<WorkerId>> {
		let rows: Vec<(String,)> = sqlx::query_as(
			r#"
			SELECT id FROM ma_workers WHERE project_run_id = ?
			"#,
		)
		.bind(project_run_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter()
			.map(|(id,)| {
				id.parse()
					.map(WorkerId)
					.map_err(|_| MultiAgentServerError::InvalidData("invalid worker ID".into()))
			})
			.collect()
	}

	#[instrument(skip(self, completion), fields(task_id = %completion.task_id))]
	async fn create_completion(&self, completion: &TaskCompletion) -> Result<()> {
		let output_json = serde_json::to_string(&completion.output)?;
		let changes_json = serde_json::to_string(&completion.changes)?;
		let verifications_json = serde_json::to_string(&completion.verifications)?;

		sqlx::query(
			r#"
			INSERT INTO ma_task_completions (
				id, task_id, worker_id, completed_at, duration_ms,
				output, changes, commit_sha, verifications
			)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(uuid::Uuid::now_v7().to_string())
		.bind(completion.task_id.to_string())
		.bind(completion.worker_id.to_string())
		.bind(completion.completed_at.to_rfc3339())
		.bind(completion.duration_ms as i64)
		.bind(&output_json)
		.bind(&changes_json)
		.bind(&completion.commit_sha)
		.bind(&verifications_json)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(task_id = %task_id))]
	async fn get_completion(&self, task_id: &TaskId) -> Result<Option<TaskCompletion>> {
		#[derive(sqlx::FromRow)]
		struct CompletionRow {
			task_id: String,
			worker_id: String,
			completed_at: String,
			duration_ms: i64,
			output: String,
			changes: String,
			commit_sha: Option<String>,
			verifications: String,
		}

		let row = sqlx::query_as::<_, CompletionRow>(
			r#"
			SELECT task_id, worker_id, completed_at, duration_ms,
				   output, changes, commit_sha, verifications
			FROM ma_task_completions
			WHERE task_id = ?
			ORDER BY completed_at DESC
			LIMIT 1
			"#,
		)
		.bind(task_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| {
			let task_id = TaskId(
				r.task_id
					.parse()
					.map_err(|_| MultiAgentServerError::InvalidData("invalid task ID".into()))?,
			);
			let worker_id = WorkerId(
				r.worker_id
					.parse()
					.map_err(|_| MultiAgentServerError::InvalidData("invalid worker ID".into()))?,
			);
			let completed_at = DateTime::parse_from_rfc3339(&r.completed_at)
				.map_err(|e| {
					MultiAgentServerError::InvalidData(format!("invalid completed_at: {e}"))
				})?
				.with_timezone(&Utc);
			let output: TaskOutput = serde_json::from_str(&r.output)?;
			let changes = serde_json::from_str(&r.changes)?;
			let verifications = serde_json::from_str(&r.verifications)?;

			Ok(TaskCompletion {
				task_id,
				worker_id,
				completed_at,
				duration_ms: r.duration_ms as u64,
				output,
				changes,
				commit_sha: r.commit_sha,
				verifications,
				context_snapshot: Default::default(),
				unblocked_tasks: Vec::new(),
			})
		})
		.transpose()
	}

	#[instrument(skip(self, snapshot), fields(checkpoint_id = %id))]
	async fn create_checkpoint(
		&self,
		id: &CheckpointId,
		project_run_id: &ProjectRunId,
		snapshot: &str,
		reason: Option<&str>,
	) -> Result<()> {
		let now = Utc::now().to_rfc3339();

		sqlx::query(
			r#"
			INSERT INTO ma_checkpoints (id, project_run_id, state_snapshot, reason, created_at)
			VALUES (?, ?, ?, ?, ?)
			"#,
		)
		.bind(id.to_string())
		.bind(project_run_id.to_string())
		.bind(snapshot)
		.bind(reason)
		.bind(&now)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[instrument(skip(self), fields(checkpoint_id = %id))]
	async fn get_checkpoint(&self, id: &CheckpointId) -> Result<Option<String>> {
		let row: Option<(String,)> = sqlx::query_as(
			r#"
			SELECT state_snapshot FROM ma_checkpoints WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row.map(|(snapshot,)| snapshot))
	}

	#[instrument(skip(self), fields(project_run_id = %project_run_id))]
	async fn list_checkpoints(&self, project_run_id: &ProjectRunId) -> Result<Vec<CheckpointId>> {
		let rows: Vec<(String,)> = sqlx::query_as(
			r#"
			SELECT id FROM ma_checkpoints WHERE project_run_id = ?
			ORDER BY created_at DESC
			"#,
		)
		.bind(project_run_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.into_iter()
			.map(|(id,)| {
				id.parse()
					.map(CheckpointId)
					.map_err(|_| MultiAgentServerError::InvalidData("invalid checkpoint ID".into()))
			})
			.collect()
	}
}
