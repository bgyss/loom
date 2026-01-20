-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- Multi-agent coordination system tables

-- Project runs
CREATE TABLE IF NOT EXISTS ma_project_runs (
    id TEXT PRIMARY KEY,
    org_id TEXT NOT NULL,
    project_path TEXT NOT NULL,
    goal TEXT NOT NULL,
    phase TEXT NOT NULL,
    status TEXT NOT NULL,
    config TEXT,        -- JSON
    metrics TEXT,       -- JSON
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_by TEXT NOT NULL
);

-- Domains
CREATE TABLE IF NOT EXISTS ma_domains (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES ma_project_runs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    file_patterns TEXT NOT NULL,  -- JSON array
    created_at TEXT NOT NULL
);

-- Planners
CREATE TABLE IF NOT EXISTS ma_planners (
    id TEXT PRIMARY KEY,
    domain_id TEXT NOT NULL REFERENCES ma_domains(id) ON DELETE CASCADE,
    parent_planner_id TEXT REFERENCES ma_planners(id),
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_active_at TEXT
);

-- Tasks
CREATE TABLE IF NOT EXISTS ma_tasks (
    id TEXT PRIMARY KEY,
    parent_id TEXT REFERENCES ma_tasks(id),
    planner_id TEXT NOT NULL REFERENCES ma_planners(id),
    priority INTEGER NOT NULL DEFAULT 0,
    task_type TEXT NOT NULL,        -- JSON
    description TEXT NOT NULL,
    specification TEXT NOT NULL,    -- JSON
    affected_files TEXT NOT NULL,   -- JSON array
    status TEXT NOT NULL,           -- JSON
    iteration INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Task dependencies
CREATE TABLE IF NOT EXISTS ma_task_dependencies (
    task_id TEXT NOT NULL REFERENCES ma_tasks(id) ON DELETE CASCADE,
    depends_on_task_id TEXT NOT NULL REFERENCES ma_tasks(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, depends_on_task_id)
);

-- Workers
CREATE TABLE IF NOT EXISTS ma_workers (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES ma_project_runs(id) ON DELETE CASCADE,
    current_task_id TEXT REFERENCES ma_tasks(id),
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_active_at TEXT
);

-- Task completions
CREATE TABLE IF NOT EXISTS ma_task_completions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES ma_tasks(id),
    worker_id TEXT NOT NULL REFERENCES ma_workers(id),
    completed_at TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    output TEXT NOT NULL,           -- JSON
    changes TEXT NOT NULL,          -- JSON array
    commit_sha TEXT,
    verifications TEXT NOT NULL     -- JSON array
);

-- File ownership (conflict prevention)
CREATE TABLE IF NOT EXISTS ma_file_ownership (
    project_run_id TEXT NOT NULL REFERENCES ma_project_runs(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    worker_id TEXT NOT NULL REFERENCES ma_workers(id),
    acquired_at TEXT NOT NULL,
    PRIMARY KEY (project_run_id, file_path)
);

-- Checkpoints
CREATE TABLE IF NOT EXISTS ma_checkpoints (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES ma_project_runs(id) ON DELETE CASCADE,
    state_snapshot TEXT NOT NULL,   -- JSON
    reason TEXT,
    created_at TEXT NOT NULL
);

-- Learnings (for context preservation)
CREATE TABLE IF NOT EXISTS ma_learnings (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES ma_project_runs(id) ON DELETE CASCADE,
    domain_id TEXT REFERENCES ma_domains(id),
    learning_type TEXT NOT NULL,
    content TEXT NOT NULL,
    source_task_id TEXT REFERENCES ma_tasks(id),
    created_at TEXT NOT NULL
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_ma_project_runs_org ON ma_project_runs(org_id);
CREATE INDEX IF NOT EXISTS idx_ma_project_runs_status ON ma_project_runs(status);
CREATE INDEX IF NOT EXISTS idx_ma_domains_project ON ma_domains(project_run_id);
CREATE INDEX IF NOT EXISTS idx_ma_planners_domain ON ma_planners(domain_id);
CREATE INDEX IF NOT EXISTS idx_ma_planners_status ON ma_planners(status);
CREATE INDEX IF NOT EXISTS idx_ma_tasks_planner ON ma_tasks(planner_id);
CREATE INDEX IF NOT EXISTS idx_ma_tasks_status ON ma_tasks(status);
CREATE INDEX IF NOT EXISTS idx_ma_tasks_priority ON ma_tasks(priority DESC);
CREATE INDEX IF NOT EXISTS idx_ma_tasks_parent ON ma_tasks(parent_id);
CREATE INDEX IF NOT EXISTS idx_ma_workers_project ON ma_workers(project_run_id);
CREATE INDEX IF NOT EXISTS idx_ma_workers_status ON ma_workers(status);
CREATE INDEX IF NOT EXISTS idx_ma_completions_task ON ma_task_completions(task_id);
CREATE INDEX IF NOT EXISTS idx_ma_completions_worker ON ma_task_completions(worker_id);
CREATE INDEX IF NOT EXISTS idx_ma_checkpoints_project ON ma_checkpoints(project_run_id);
CREATE INDEX IF NOT EXISTS idx_ma_learnings_project ON ma_learnings(project_run_id);
CREATE INDEX IF NOT EXISTS idx_ma_learnings_domain ON ma_learnings(domain_id);
