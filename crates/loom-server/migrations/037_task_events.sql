-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- Task events log for persistence and replay

CREATE TABLE IF NOT EXISTS ma_task_events (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    task_id TEXT,
    planner_id TEXT,
    worker_id TEXT,
    payload TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    processed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_ma_task_events_timestamp ON ma_task_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_ma_task_events_project_run ON ma_task_events(project_run_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_ma_task_events_planner ON ma_task_events(planner_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_ma_task_events_task ON ma_task_events(task_id);
CREATE INDEX IF NOT EXISTS idx_ma_task_events_unprocessed ON ma_task_events(processed) WHERE processed = 0;

-- Persistent subscriptions for event streaming
CREATE TABLE IF NOT EXISTS ma_event_subscriptions (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL,
    subscriber_type TEXT NOT NULL,
    subscriber_id TEXT NOT NULL,
    filter TEXT NOT NULL,
    last_event_id TEXT,
    webhook_url TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ma_event_subscriptions_project ON ma_event_subscriptions(project_run_id);
CREATE INDEX IF NOT EXISTS idx_ma_event_subscriptions_subscriber ON ma_event_subscriptions(subscriber_type, subscriber_id);

-- Deferred events for back-pressure handling
CREATE TABLE IF NOT EXISTS ma_deferred_events (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL,
    planner_id TEXT NOT NULL,
    event_payload TEXT NOT NULL,
    deferred_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ma_deferred_next_retry ON ma_deferred_events(next_retry_at);
CREATE INDEX IF NOT EXISTS idx_ma_deferred_planner ON ma_deferred_events(planner_id);
