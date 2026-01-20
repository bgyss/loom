<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Loom

> [!CAUTION]
> **Loom is a research project. If your name is not Geoffrey Huntley then do not use.**
>
> This software is experimental, unstable, and under active development. APIs will change without notice. Features may be incomplete or broken. There is no support, no documentation guarantees, and no warranty of any kind. Use at your own risk.

## Overview

Loom is an AI-powered coding agent built in Rust. It provides a REPL interface for interacting with LLM-powered agents that can execute tools to perform file system operations, code analysis, and other development tasks.

The system is designed around three core principles:

1. **Modularity** - Clean separation between core abstractions, LLM providers, and tools
2. **Extensibility** - Easy addition of new LLM providers and tools via trait implementations
3. **Reliability** - Robust error handling with retry mechanisms and structured logging

## Architecture

Loom is organized as a Cargo workspace with 30+ crates:

```
loom/
├── crates/
│   ├── loom-core/           # Core abstractions, state machine, types
│   ├── loom-server/         # HTTP API server with LLM proxy
│   ├── loom-cli/            # Command-line interface
│   ├── loom-thread/         # Conversation persistence and sync
│   ├── loom-tools/          # Agent tool implementations
│   ├── loom-llm-*/          # LLM provider integrations
│   ├── loom-auth*/          # Authentication and authorization
│   ├── loom-tui-*/          # Terminal UI components
│   ├── loom-multi-agent*/   # Multi-agent coordination system
│   └── ...                  # Many more specialized crates
├── web/
│   └── loom-web/            # Svelte 5 web frontend
├── specs/                   # Design specifications
└── infra/                   # Nix/K8s infrastructure
```

### Key Components

| Component | Description |
|-----------|-------------|
| **Core Agent** | State machine for conversation flow and tool orchestration |
| **Multi-Agent System** | Hierarchical coordination for 10-100+ concurrent agents on a single codebase |
| **LLM Proxy** | Server-side proxy architecture - API keys never leave the server |
| **Tool System** | Registry and execution framework for agent capabilities |
| **Weaver** | Remote execution environments via Kubernetes pods |
| **Thread System** | Conversation persistence with FTS5 search |
| **Analytics** | PostHog-style product analytics with identity resolution |
| **Auth** | OAuth, magic links, ABAC authorization |
| **Feature Flags** | Runtime feature toggles, experiments, and kill switches |

### Server-Side LLM Proxy

Loom uses a server-side proxy architecture for all LLM interactions:

```
┌─────────────┐      HTTP       ┌─────────────┐     Provider API    ┌─────────────┐
│  loom-cli   │ ───────────────▶│ loom-server │ ──────────────────▶ │  Anthropic  │
│             │ /proxy/{provider}│             │                     │   OpenAI    │
│ ProxyLlm-   │  /complete      │  LlmService │                     │    etc.     │
│ Client      │  /stream        │             │                     │             │
└─────────────┘ ◀─────────────  └─────────────┘ ◀────────────────── └─────────────┘
                  SSE stream                        SSE stream
```

API keys are stored server-side only. Clients communicate through the proxy.

### Multi-Agent Coordination

The multi-agent system enables 10-100+ concurrent agents to work on a single codebase for extended periods:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Orchestrator                             │
│  (Singleton coordinator - manages project state & lifecycle)    │
└─────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┼───────────────┐
                ▼               ▼               ▼
        ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
        │   Domain    │ │   Domain    │ │   Domain    │
        │   Planner   │ │   Planner   │ │   Planner   │
        │  (backend)  │ │ (frontend)  │ │   (infra)   │
        └─────────────┘ └─────────────┘ └─────────────┘
                │               │               │
                ▼               ▼               ▼
        ┌─────────────────────────────────────────────┐
        │              Task Queue                      │
        │  (Priority ordering, dependency resolution)  │
        └─────────────────────────────────────────────┘
                                │
        ┌───────────┬───────────┼───────────┬───────────┐
        ▼           ▼           ▼           ▼           ▼
    ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
    │Worker 1│ │Worker 2│ │Worker 3│ │Worker 4│ │Worker N│
    └────────┘ └────────┘ └────────┘ └────────┘ └────────┘
                                │
                                ▼
                        ┌─────────────┐
                        │    Judge    │
                        │    Agent    │
                        │ (Evaluates) │
                        └─────────────┘
```

**Crates:**
- `loom-multi-agent-core` - Core types (Task, Domain, Verdict, etc.)
- `loom-multi-agent-events` - Event bus for planner wake-up
- `loom-server-multi-agent` - Repository, task queue, file ownership
- `loom-multi-agent` - Agent implementations

See [specs/multi-agent-coordination-system.md](specs/multi-agent-coordination-system.md) for the full design.

## Building

### With Nix (Preferred)

Uses cargo2nix for reproducible builds with per-crate caching:

```bash
# Build CLI
nix build .#loom-cli-c2n

# Build server
nix build .#loom-server-c2n

# Build any crate
nix build .#<crate-name>-c2n
```

### With Cargo (Development)

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Full check
make check
```

## Specifications

Design documentation lives in `specs/`. See `specs/README.md` for a complete index organized by category:

- Core Architecture
- LLM Integration
- Configuration & Security
- Analytics & Experimentation
- Editor Integration
- Remote Execution (Weaver)
- And more...

## License

Proprietary. Copyright (c) 2025 Geoffrey Huntley. All rights reserved.
