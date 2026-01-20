-- Context Management System Tables
-- Migration 036: Add tables for context management (memories, verifications, drift signals, etc.)

-- Memory items
CREATE TABLE IF NOT EXISTS ctx_memory_items (
    id TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL,
    content_type TEXT NOT NULL,
    content TEXT NOT NULL,  -- JSON
    created_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    relevance_score REAL NOT NULL DEFAULT 1.0,
    tags TEXT NOT NULL DEFAULT '[]',  -- JSON array
    source_type TEXT NOT NULL,
    source_id TEXT,
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE
);

-- Verification history
CREATE TABLE IF NOT EXISTS ctx_verifications (
    id TEXT PRIMARY KEY,
    claim_type TEXT NOT NULL,
    assertion TEXT NOT NULL,
    passed INTEGER NOT NULL,
    output TEXT NOT NULL,
    verified_at TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    task_id TEXT,
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE
);

-- Drift signals
CREATE TABLE IF NOT EXISTS ctx_drift_signals (
    id TEXT PRIMARY KEY,
    signal_type TEXT NOT NULL,
    details TEXT NOT NULL,  -- JSON
    detected_at TEXT NOT NULL,
    corrected_at TEXT,
    correction_action TEXT,  -- JSON
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE
);

-- Strategy performance
CREATE TABLE IF NOT EXISTS ctx_strategy_performance (
    task_type TEXT NOT NULL,
    strategy TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    successes INTEGER NOT NULL DEFAULT 0,
    avg_duration_ms INTEGER,
    avg_tokens_used INTEGER,
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    PRIMARY KEY (org_id, task_type, strategy)
);

-- Context snapshots
CREATE TABLE IF NOT EXISTS ctx_snapshots (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    assembled_at TEXT NOT NULL,
    total_tokens INTEGER NOT NULL,
    sections TEXT NOT NULL,  -- JSON
    resource_usage TEXT NOT NULL,  -- JSON
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE
);

-- Indexes for memory items
CREATE INDEX IF NOT EXISTS idx_ctx_memory_items_type ON ctx_memory_items(memory_type);
CREATE INDEX IF NOT EXISTS idx_ctx_memory_items_org ON ctx_memory_items(org_id);
CREATE INDEX IF NOT EXISTS idx_ctx_memory_items_relevance ON ctx_memory_items(relevance_score DESC);
CREATE INDEX IF NOT EXISTS idx_ctx_memory_items_last_accessed ON ctx_memory_items(last_accessed DESC);

-- Indexes for verifications
CREATE INDEX IF NOT EXISTS idx_ctx_verifications_task ON ctx_verifications(task_id);
CREATE INDEX IF NOT EXISTS idx_ctx_verifications_org ON ctx_verifications(org_id);

-- Indexes for drift signals
CREATE INDEX IF NOT EXISTS idx_ctx_drift_signals_type ON ctx_drift_signals(signal_type);
CREATE INDEX IF NOT EXISTS idx_ctx_drift_signals_org ON ctx_drift_signals(org_id);
CREATE INDEX IF NOT EXISTS idx_ctx_drift_signals_active ON ctx_drift_signals(org_id, corrected_at) WHERE corrected_at IS NULL;

-- Indexes for snapshots
CREATE INDEX IF NOT EXISTS idx_ctx_snapshots_task ON ctx_snapshots(task_id);
CREATE INDEX IF NOT EXISTS idx_ctx_snapshots_org ON ctx_snapshots(org_id);
