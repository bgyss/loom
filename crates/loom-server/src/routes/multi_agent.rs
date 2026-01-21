// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Multi-agent coordination HTTP handlers.
//!
//! Implements API endpoints for the multi-agent coordination system:
//! - Create/get/list project runs
//! - List tasks and workers
//! - Create checkpoints
//! - Pause/resume projects
//! - Stream events (SSE)
//! - Event history

use std::convert::Infallible;
use std::time::Duration;

use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::{
		sse::{Event, KeepAlive, Sse},
		IntoResponse,
	},
	Json,
};
use futures::stream::{self, Stream};

pub use loom_server_api::multi_agent::*;

use crate::{
	api::AppState,
	auth_middleware::RequireAuth,
	i18n::{resolve_user_locale, t},
	impl_api_error_response,
};

impl_api_error_response!(MultiAgentErrorResponse);

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/multi-agent/projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 201, description = "Project run created", body = ProjectRunResponse),
        (status = 400, description = "Invalid request", body = MultiAgentErrorResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// POST /api/orgs/{org_id}/multi-agent/projects - Create a new project run.
pub async fn create_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(org_id): Path<String>,
	Json(request): Json<CreateProjectRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	if request.project_path.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(MultiAgentErrorResponse {
				error: "invalid_request".to_string(),
				message: "Project path is required".to_string(),
			}),
		)
			.into_response();
	}

	if request.goal.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(MultiAgentErrorResponse {
				error: "invalid_request".to_string(),
				message: "Goal is required".to_string(),
			}),
		)
			.into_response();
	}

	let response = ProjectRunResponse {
		id: format!("run-{}", uuid::Uuid::now_v7()),
		org_id: org_id.clone(),
		project_path: request.project_path,
		goal: request.goal,
		phase: "exploration".to_string(),
		status: "initializing".to_string(),
		metrics: ProjectMetricsResponse::default(),
		started_at: chrono::Utc::now(),
		completed_at: None,
		created_by: current_user.user.id.to_string(),
	};

	(StatusCode::CREATED, Json(response)).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}",
    responses(
        (status = 200, description = "Project run details", body = ProjectRunResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// GET /api/orgs/{org_id}/multi-agent/projects/{run_id} - Get project run details.
pub async fn get_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, run_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	(
		StatusCode::NOT_FOUND,
		Json(MultiAgentErrorResponse {
			error: "not_found".to_string(),
			message: format!("Project run {} not found", run_id),
		}),
	)
		.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/multi-agent/projects",
    responses(
        (status = 200, description = "List of project runs", body = ListProjectRunsResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// GET /api/orgs/{org_id}/multi-agent/projects - List project runs.
pub async fn list_projects(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(org_id): Path<String>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	Json(ListProjectRunsResponse { projects: vec![] }).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/tasks",
    responses(
        (status = 200, description = "List of tasks", body = ListTasksResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// GET /api/orgs/{org_id}/multi-agent/projects/{run_id}/tasks - List tasks.
pub async fn list_tasks(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, run_id)): Path<(String, String)>,
	Query(query): Query<ListTasksQuery>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	Json(ListTasksResponse { tasks: vec![] }).into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/workers",
    responses(
        (status = 200, description = "List of workers", body = ListWorkersResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// GET /api/orgs/{org_id}/multi-agent/projects/{run_id}/workers - List workers.
pub async fn list_workers(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, run_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	Json(ListWorkersResponse { workers: vec![] }).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/checkpoint",
    request_body = CreateCheckpointRequest,
    responses(
        (status = 201, description = "Checkpoint created", body = CheckpointResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// POST /api/orgs/{org_id}/multi-agent/projects/{run_id}/checkpoint - Create checkpoint.
pub async fn create_checkpoint(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, run_id)): Path<(String, String)>,
	Json(request): Json<CreateCheckpointRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let response = CheckpointResponse {
		id: format!("checkpoint-{}", uuid::Uuid::now_v7()),
		project_run_id: run_id,
		created_at: chrono::Utc::now(),
		reason: request.reason,
	};

	(StatusCode::CREATED, Json(response)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/pause",
    responses(
        (status = 200, description = "Project paused", body = MultiAgentSuccessResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// POST /api/orgs/{org_id}/multi-agent/projects/{run_id}/pause - Pause project.
pub async fn pause_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, run_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	Json(MultiAgentSuccessResponse {
		message: format!("Project {} paused", run_id),
	})
	.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/resume",
    responses(
        (status = 200, description = "Project resumed", body = MultiAgentSuccessResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// POST /api/orgs/{org_id}/multi-agent/projects/{run_id}/resume - Resume project.
pub async fn resume_project(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((org_id, run_id)): Path<(String, String)>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	Json(MultiAgentSuccessResponse {
		message: format!("Project {} resumed", run_id),
	})
	.into_response()
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/events",
    responses(
        (status = 200, description = "SSE event stream"),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// GET /api/orgs/{org_id}/multi-agent/projects/{run_id}/events - Stream events via SSE.
pub async fn event_stream(
	State(_state): State<AppState>,
	RequireAuth(_current_user): RequireAuth,
	Path((_org_id, run_id)): Path<(String, String)>,
	Query(_params): Query<EventStreamQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
	let stream = stream::unfold(0u64, move |counter| {
		let run_id = run_id.clone();
		async move {
			tokio::time::sleep(Duration::from_secs(1)).await;

			let event_data = serde_json::json!({
				"run_id": run_id,
				"event": "heartbeat",
				"counter": counter,
			});

			let event = Event::default()
				.data(event_data.to_string())
				.id(counter.to_string())
				.event("heartbeat");

			Some((Ok(event), counter + 1))
		}
	});

	Sse::new(stream).keep_alive(KeepAlive::default())
}

#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/multi-agent/projects/{run_id}/events/history",
    responses(
        (status = 200, description = "Event history", body = EventHistoryResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse),
        (status = 404, description = "Project not found", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// GET /api/orgs/{org_id}/multi-agent/projects/{run_id}/events/history - Get event history.
pub async fn event_history(
	State(_state): State<AppState>,
	RequireAuth(_current_user): RequireAuth,
	Path((_org_id, _run_id)): Path<(String, String)>,
	Query(_params): Query<EventHistoryQuery>,
) -> impl IntoResponse {
	Json(EventHistoryResponse {
		events: vec![],
		total: 0,
		has_more: false,
	})
	.into_response()
}

#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/multi-agent/subscriptions",
    request_body = CreateSubscriptionRequest,
    responses(
        (status = 201, description = "Subscription created", body = SubscriptionResponse),
        (status = 400, description = "Invalid request", body = MultiAgentErrorResponse),
        (status = 401, description = "Not authenticated", body = MultiAgentErrorResponse)
    ),
    tag = "multi-agent"
)]
/// POST /api/orgs/{org_id}/multi-agent/subscriptions - Create event subscription.
pub async fn create_subscription(
	State(_state): State<AppState>,
	RequireAuth(_current_user): RequireAuth,
	Path(_org_id): Path<String>,
	Json(request): Json<CreateSubscriptionRequest>,
) -> impl IntoResponse {
	let response = SubscriptionResponse {
		subscription_id: format!("sub-{}", uuid::Uuid::now_v7()),
		filter: request.filter,
		created_at: chrono::Utc::now(),
	};

	(StatusCode::CREATED, Json(response)).into_response()
}
