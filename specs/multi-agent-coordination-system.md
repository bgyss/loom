<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Multi-Agent Coordination System Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-19

---

## 1. Overview

### Purpose

Provide a hierarchical multi-agent coordination system that enables hundreds of agents to work together on a single codebase for extended periods (days to weeks), making real progress on ambitious projects.

### Goals

- **Hierarchical coordination**: Role-based architecture with planners and workers to avoid flat coordination bottlenecks
- **Scalable parallelism**: Support 10-100+ concurrent agents working on a single codebase
- **Planner wake-up**: Event-driven planner activation when tasks complete to plan next steps
- **Context preservation**: Avoid tunnel vision and drift without requiring periodic fresh starts
- **Real progress**: Ensure agents make meaningful progress rather than churning on safe, small changes

### Non-Goals

- Distributed multi-codebase coordination (single codebase focus)
- Real-time collaborative editing (async task-based model)
- Human-in-the-loop for every decision (autonomous operation with checkpoints)

### Background

Research from Cursor's autonomous coding experiments reveals:

1. **Flat coordination fails at scale**: Agents with equal status using lock-based coordination slow down to 2-3x throughput regardless of agent count
2. **Hierarchy enables scale**: Role-based architecture (planners create tasks, workers execute) solves coordination problems
3. **Structure prevents drift**: Without hierarchy, agents become risk-averse and make small, safe changes instead of ambitious progress
4. **Model specialization matters**: Different models excel at different roles (planning vs execution)

---

## 2. Architecture

### Role Hierarchy

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           Orchestrator (Singleton)                            │
│  - Global state management                                                    │
│  - Resource allocation                                                        │
│  - Health monitoring                                                          │
│  - Checkpoint coordination                                                    │
└──────────────────────────────────────────────────────────────────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    ▼                 ▼                 ▼
         ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
         │  Domain Planner  │ │  Domain Planner  │ │  Domain Planner  │
         │  (Frontend)      │ │  (Backend)       │ │  (Infrastructure)│
         │                  │ │                  │ │                  │
         │  - Area expert   │ │  - Area expert   │ │  - Area expert   │
         │  - Task creation │ │  - Task creation │ │  - Task creation │
         │  - Sub-planner   │ │  - Sub-planner   │ │  - Sub-planner   │
         │    spawning      │ │    spawning      │ │    spawning      │
         └────────┬─────────┘ └────────┬─────────┘ └────────┬─────────┘
                  │                    │                    │
     ┌────────────┼────────────┐      │      ┌─────────────┼────────────┐
     ▼            ▼            ▼      ▼      ▼             ▼            ▼
┌─────────┐ ┌─────────┐ ┌─────────┐ ... ┌─────────┐ ┌─────────┐ ┌─────────┐
│ Worker  │ │ Worker  │ │ Worker  │     │ Worker  │ │ Worker  │ │ Worker  │
│         │ │         │ │         │     │         │ │         │ │         │
│ - Task  │ │ - Task  │ │ - Task  │     │ - Task  │ │ - Task  │ │ - Task  │
│   exec  │ │   exec  │ │   exec  │     │   exec  │ │   exec  │ │   exec  │
│ - No    │ │ - No    │ │ - No    │     │ - No    │ │ - No    │ │ - No    │
│   coord │ │   coord │ │   coord │     │   coord │ │   coord │ │   coord │
└─────────┘ └─────────┘ └─────────┘     └─────────┘ └─────────┘ └─────────┘
                                      │
                                      ▼
                        ┌──────────────────────────┐
                        │     Judge Agent          │
                        │  - Completion evaluation │
                        │  - Quality assessment    │
                        │  - Iteration decisions   │
                        └──────────────────────────┘
```

### Role Definitions

#### Orchestrator

The singleton coordinator responsible for global state and resource management.

```rust
pub struct Orchestrator {
    /// Global project state
    project_state: Arc<RwLock<ProjectState>>,
    /// Active planners by domain
    planners: HashMap<DomainId, Arc<DomainPlanner>>,
    /// Task queue with priority
    task_queue: Arc<TaskQueue>,
    /// Worker pool
    worker_pool: WorkerPool,
    /// Event bus for coordination
    event_bus: Arc<EventBus>,
    /// Checkpoint manager
    checkpoint_manager: CheckpointManager,
}

impl Orchestrator {
    /// Initialize with project analysis
    pub async fn initialize(project_path: &Path) -> Result<Self>;

    /// Spawn domain planners based on project structure
    pub async fn spawn_planners(&mut self) -> Result<Vec<DomainId>>;

    /// Main coordination loop
    pub async fn run(&mut self) -> Result<ProjectOutcome>;

    /// Handle global events
    pub async fn handle_event(&mut self, event: OrchestratorEvent) -> Result<()>;

    /// Create checkpoint for recovery
    pub async fn checkpoint(&self) -> Result<CheckpointId>;
}
```

#### Domain Planner

Planners continuously explore specific areas of the codebase and generate tasks.

```rust
pub struct DomainPlanner {
    /// Planner identity
    id: PlannerId,
    /// Domain this planner is responsible for
    domain: Domain,
    /// Current understanding of the domain
    domain_model: DomainModel,
    /// LLM client for planning
    llm: Arc<dyn LlmClient>,
    /// Event subscription
    event_rx: mpsc::Receiver<PlannerEvent>,
    /// Task output channel
    task_tx: mpsc::Sender<Task>,
    /// Sub-planners spawned by this planner
    sub_planners: Vec<PlannerId>,
    /// Context manager for drift prevention
    context_manager: ContextManager,
}

#[derive(Debug, Clone)]
pub struct Domain {
    /// Domain identifier
    pub id: DomainId,
    /// Human-readable name
    pub name: String,
    /// File patterns this domain covers
    pub file_patterns: Vec<GlobPattern>,
    /// Dependencies on other domains
    pub dependencies: Vec<DomainId>,
}

impl DomainPlanner {
    /// Create a new planner for a domain
    pub fn new(domain: Domain, llm: Arc<dyn LlmClient>) -> Self;

    /// Main planning loop - wakes on events
    pub async fn run(&mut self) -> Result<()>;

    /// Handle task completion notification
    pub async fn on_task_completed(&mut self, completion: TaskCompletion) -> Result<Vec<Task>>;

    /// Explore domain and generate initial tasks
    pub async fn explore_and_plan(&mut self) -> Result<Vec<Task>>;

    /// Spawn a sub-planner for a specific area
    pub async fn spawn_sub_planner(&mut self, area: &str) -> Result<PlannerId>;
}
```

#### Worker

Workers focus exclusively on task completion without coordinating with other workers.

```rust
pub struct Worker {
    /// Worker identity
    id: WorkerId,
    /// LLM client for execution
    llm: Arc<dyn LlmClient>,
    /// Tool registry
    tools: Arc<ToolRegistry>,
    /// Current task (if any)
    current_task: Option<Task>,
    /// Completion channel
    completion_tx: mpsc::Sender<TaskCompletion>,
    /// Context manager
    context_manager: ContextManager,
}

impl Worker {
    /// Execute a task to completion
    pub async fn execute(&mut self, task: Task) -> Result<TaskCompletion>;

    /// Push changes when task is done
    pub async fn commit_changes(&self, task: &Task) -> Result<CommitInfo>;
}
```

#### Judge Agent

Evaluates completion and determines whether to continue iterating.

```rust
pub struct JudgeAgent {
    /// LLM client for evaluation
    llm: Arc<dyn LlmClient>,
    /// Evaluation criteria
    criteria: EvaluationCriteria,
}

#[derive(Debug, Clone)]
pub struct EvaluationCriteria {
    /// Required test pass rate
    pub min_test_pass_rate: f64,
    /// Required code quality score
    pub min_quality_score: f64,
    /// Maximum iterations before escalation
    pub max_iterations: u32,
}

impl JudgeAgent {
    /// Evaluate task completion
    pub async fn evaluate(&self, completion: &TaskCompletion) -> Result<Verdict>;

    /// Determine if task needs iteration
    pub async fn needs_iteration(&self, completion: &TaskCompletion) -> Result<bool>;
}

#[derive(Debug, Clone)]
pub enum Verdict {
    /// Task completed successfully
    Approved { feedback: String },
    /// Task needs more work
    NeedsIteration { feedback: String, suggestions: Vec<String> },
    /// Task cannot be completed, escalate
    Escalate { reason: String },
}
```

---

## 3. Task System

### Task Definition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Parent task (if subtask)
    pub parent_id: Option<TaskId>,
    /// Planner that created this task
    pub created_by: PlannerId,
    /// Task priority (higher = more important)
    pub priority: i32,
    /// Task type
    pub task_type: TaskType,
    /// Human-readable description
    pub description: String,
    /// Detailed specification
    pub specification: TaskSpecification,
    /// Files this task may modify
    pub affected_files: Vec<PathBuf>,
    /// Dependencies on other tasks
    pub dependencies: Vec<TaskId>,
    /// Current status
    pub status: TaskStatus,
    /// Deadline (if any)
    pub deadline: Option<DateTime<Utc>>,
    /// Context snapshot at creation
    pub context_snapshot: ContextSnapshot,
    /// Iteration count
    pub iteration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// Implement new feature
    Feature { acceptance_criteria: Vec<String> },
    /// Fix a bug
    BugFix { reproduction_steps: Vec<String>, expected_behavior: String },
    /// Refactor existing code
    Refactor { goals: Vec<String>, constraints: Vec<String> },
    /// Write or update tests
    Test { coverage_target: Option<f64> },
    /// Documentation
    Documentation { sections: Vec<String> },
    /// Code review
    Review { changes: Vec<PathBuf> },
    /// Exploration (no code changes)
    Exploration { questions: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpecification {
    /// Detailed requirements
    pub requirements: Vec<String>,
    /// Success criteria (verifiable)
    pub success_criteria: Vec<VerifiableCriterion>,
    /// Constraints
    pub constraints: Vec<String>,
    /// Hints from planner
    pub hints: Vec<String>,
    /// Relevant context
    pub context: Vec<ContextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCriterion {
    /// Description
    pub description: String,
    /// Verification method
    pub verification: VerificationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// Run tests
    TestPass { test_pattern: String },
    /// Check compilation
    CompilationSuccess,
    /// Lint check
    LintPass { linter: String },
    /// Type check
    TypeCheck,
    /// Custom command
    Command { cmd: String, expected_exit_code: i32 },
    /// File exists
    FileExists { path: PathBuf },
    /// Pattern match in file
    PatternMatch { path: PathBuf, pattern: String },
}
```

### Task Status

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Waiting for dependencies
    Blocked { waiting_on: Vec<TaskId> },
    /// Ready to be picked up
    Pending,
    /// Assigned to a worker
    Assigned { worker_id: WorkerId, assigned_at: DateTime<Utc> },
    /// Currently being executed
    InProgress { worker_id: WorkerId, started_at: DateTime<Utc>, progress: TaskProgress },
    /// Awaiting judge evaluation
    AwaitingReview { completed_at: DateTime<Utc> },
    /// Needs iteration based on judge feedback
    NeedsIteration { feedback: String, iteration: u32 },
    /// Successfully completed
    Completed { completed_at: DateTime<Utc>, output: TaskOutput },
    /// Failed permanently
    Failed { failed_at: DateTime<Utc>, error: String },
    /// Cancelled
    Cancelled { cancelled_at: DateTime<Utc>, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Current step
    pub current_step: String,
    /// Progress percentage (0-100)
    pub percentage: u8,
    /// Files modified so far
    pub files_modified: Vec<PathBuf>,
    /// Verification results so far
    pub verifications: Vec<VerificationResult>,
}
```

### Task Completion

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletion {
    /// Task that was completed
    pub task_id: TaskId,
    /// Worker that completed it
    pub worker_id: WorkerId,
    /// Completion timestamp
    pub completed_at: DateTime<Utc>,
    /// Duration
    pub duration: Duration,
    /// Output
    pub output: TaskOutput,
    /// Files changed
    pub changes: Vec<FileChange>,
    /// Commit (if any)
    pub commit: Option<CommitInfo>,
    /// Verification results
    pub verifications: Vec<VerificationResult>,
    /// Context at completion (for planner wake-up)
    pub context_snapshot: ContextSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Summary of what was done
    pub summary: String,
    /// Detailed notes
    pub notes: Vec<String>,
    /// Artifacts produced
    pub artifacts: Vec<Artifact>,
    /// Suggested follow-up tasks
    pub suggested_tasks: Vec<TaskSuggestion>,
    /// Learnings for future context
    pub learnings: Vec<Learning>,
}
```

---

## 4. Planner Wake-up System

### Event-Driven Activation

Planners don't poll - they wake up in response to specific events.

```rust
#[derive(Debug, Clone)]
pub enum PlannerEvent {
    /// A task assigned by this planner completed
    TaskCompleted(TaskCompletion),
    /// A dependent domain changed
    DomainChanged { domain_id: DomainId, changes: Vec<PathBuf> },
    /// Orchestrator requested planning
    PlanningRequested { reason: String },
    /// Resource became available
    ResourceAvailable { resource_type: ResourceType },
    /// Checkpoint restored (planner should re-orient)
    CheckpointRestored { checkpoint_id: CheckpointId },
    /// Sub-planner completed its scope
    SubPlannerCompleted { planner_id: PlannerId, summary: String },
    /// Iteration feedback from judge
    IterationFeedback { task_id: TaskId, feedback: String },
    /// Shutdown requested
    Shutdown,
}

impl DomainPlanner {
    /// Main loop - waits for events
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let event = self.event_rx.recv().await?;

            match event {
                PlannerEvent::TaskCompleted(completion) => {
                    // Wake up to plan next steps
                    let new_tasks = self.on_task_completed(completion).await?;
                    for task in new_tasks {
                        self.task_tx.send(task).await?;
                    }
                }
                PlannerEvent::DomainChanged { domain_id, changes } => {
                    // Re-evaluate domain model
                    self.update_domain_model(&changes).await?;
                    // Possibly generate new tasks
                    let tasks = self.react_to_changes(&changes).await?;
                    for task in tasks {
                        self.task_tx.send(task).await?;
                    }
                }
                PlannerEvent::IterationFeedback { task_id, feedback } => {
                    // Create iteration task with feedback context
                    let iteration_task = self.create_iteration_task(task_id, &feedback).await?;
                    self.task_tx.send(iteration_task).await?;
                }
                PlannerEvent::Shutdown => break,
                // ... other events
            }

            // Refresh context after handling event
            self.context_manager.refresh().await?;
        }
        Ok(())
    }

    /// Handle task completion - the key wake-up moment
    async fn on_task_completed(&mut self, completion: TaskCompletion) -> Result<Vec<Task>> {
        let mut new_tasks = Vec::new();

        // 1. Absorb learnings from completion
        self.absorb_learnings(&completion).await?;

        // 2. Check if completion unblocks other tasks
        let unblocked = self.find_unblocked_tasks(&completion).await?;

        // 3. Analyze changes and determine follow-up needs
        let follow_ups = self.analyze_for_follow_ups(&completion).await?;
        new_tasks.extend(follow_ups);

        // 4. Check if domain goals are met or need new planning
        if self.should_expand_scope(&completion).await? {
            let expansion_tasks = self.expand_scope().await?;
            new_tasks.extend(expansion_tasks);
        }

        // 5. Update domain model with new knowledge
        self.domain_model.incorporate(&completion).await?;

        Ok(new_tasks)
    }
}
```

### Task Dependency Resolution

```rust
pub struct TaskQueue {
    /// All tasks by ID
    tasks: HashMap<TaskId, Task>,
    /// Ready queue (no unmet dependencies)
    ready: BinaryHeap<PrioritizedTask>,
    /// Blocked tasks by blocking task
    blocked_by: HashMap<TaskId, HashSet<TaskId>>,
    /// Event channels to planners
    planner_events: HashMap<PlannerId, mpsc::Sender<PlannerEvent>>,
}

impl TaskQueue {
    /// Add a new task
    pub async fn enqueue(&mut self, task: Task) -> Result<()> {
        let task_id = task.id.clone();
        let planner_id = task.created_by.clone();

        if task.dependencies.is_empty() {
            // Immediately ready
            self.ready.push(PrioritizedTask::from(&task));
        } else {
            // Track blocking relationships
            for dep in &task.dependencies {
                self.blocked_by.entry(dep.clone())
                    .or_default()
                    .insert(task_id.clone());
            }
        }

        self.tasks.insert(task_id, task);
        Ok(())
    }

    /// Mark task as completed and notify planner
    pub async fn complete(&mut self, completion: TaskCompletion) -> Result<()> {
        let task_id = &completion.task_id;

        // Get the task and its planner
        let task = self.tasks.get(task_id).ok_or(Error::TaskNotFound)?;
        let planner_id = task.created_by.clone();

        // Unblock dependent tasks
        if let Some(blocked_tasks) = self.blocked_by.remove(task_id) {
            for blocked_id in blocked_tasks {
                if let Some(blocked_task) = self.tasks.get_mut(&blocked_id) {
                    // Remove this dependency
                    blocked_task.dependencies.retain(|d| d != task_id);

                    // If no more dependencies, move to ready queue
                    if blocked_task.dependencies.is_empty() {
                        self.ready.push(PrioritizedTask::from(blocked_task));
                    }
                }
            }
        }

        // Notify planner - THIS IS THE WAKE-UP
        if let Some(tx) = self.planner_events.get(&planner_id) {
            tx.send(PlannerEvent::TaskCompleted(completion)).await?;
        }

        Ok(())
    }
}
```

---

## 5. Conflict Prevention

### File Ownership Model

Instead of locks, use exclusive file ownership during task execution.

```rust
pub struct FileOwnership {
    /// Files currently owned by workers
    owned: HashMap<PathBuf, WorkerId>,
    /// Pending ownership requests
    pending: HashMap<PathBuf, Vec<(WorkerId, oneshot::Sender<bool>)>>,
}

impl FileOwnership {
    /// Request ownership of files for a task
    /// Returns only when ownership is granted or denied
    pub async fn request(&mut self, worker_id: WorkerId, files: &[PathBuf]) -> Result<FileOwnershipGuard> {
        // Check for conflicts
        let conflicts: Vec<_> = files.iter()
            .filter(|f| self.owned.contains_key(*f))
            .collect();

        if conflicts.is_empty() {
            // Grant immediately
            for file in files {
                self.owned.insert(file.clone(), worker_id.clone());
            }
            Ok(FileOwnershipGuard::new(self, worker_id, files.to_vec()))
        } else {
            // Queue the request
            let (tx, rx) = oneshot::channel();
            for file in &conflicts {
                self.pending.entry((*file).clone())
                    .or_default()
                    .push((worker_id.clone(), tx.clone()));
            }

            // Wait for ownership
            if rx.await? {
                Ok(FileOwnershipGuard::new(self, worker_id, files.to_vec()))
            } else {
                Err(Error::OwnershipDenied)
            }
        }
    }

    /// Release ownership when task completes
    pub fn release(&mut self, worker_id: &WorkerId, files: &[PathBuf]) {
        for file in files {
            if self.owned.get(file) == Some(worker_id) {
                self.owned.remove(file);

                // Grant to next pending request
                if let Some(pending) = self.pending.get_mut(file) {
                    if let Some((next_worker, tx)) = pending.pop() {
                        self.owned.insert(file.clone(), next_worker);
                        let _ = tx.send(true);
                    }
                }
            }
        }
    }
}
```

### Task Conflict Detection

```rust
impl TaskQueue {
    /// Check if a new task conflicts with existing tasks
    pub fn check_conflicts(&self, task: &Task) -> Vec<TaskConflict> {
        let mut conflicts = Vec::new();

        for (existing_id, existing) in &self.tasks {
            if existing.status.is_active() {
                // Check file overlap
                let file_overlap: Vec<_> = task.affected_files.iter()
                    .filter(|f| existing.affected_files.contains(f))
                    .cloned()
                    .collect();

                if !file_overlap.is_empty() {
                    conflicts.push(TaskConflict {
                        task_id: existing_id.clone(),
                        conflict_type: ConflictType::FileOverlap(file_overlap),
                    });
                }
            }
        }

        conflicts
    }
}
```

---

## 6. Model Specialization

### Role-Based Model Selection

```rust
pub struct ModelSelector {
    /// Model configurations by role
    role_models: HashMap<AgentRole, ModelConfig>,
    /// Fallback model
    default_model: ModelConfig,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum AgentRole {
    /// High-level planning and coordination
    Planner,
    /// Code implementation
    Worker,
    /// Quality evaluation
    Judge,
    /// Code exploration and analysis
    Explorer,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model identifier
    pub model_id: String,
    /// Provider
    pub provider: LlmProvider,
    /// Temperature
    pub temperature: f32,
    /// Max tokens
    pub max_tokens: u32,
    /// System prompt customization
    pub system_prompt_suffix: Option<String>,
}

impl ModelSelector {
    /// Get model for a specific role
    pub fn for_role(&self, role: AgentRole) -> &ModelConfig {
        self.role_models.get(&role).unwrap_or(&self.default_model)
    }

    /// Recommended configuration based on Cursor research
    pub fn recommended() -> Self {
        let mut role_models = HashMap::new();

        // Planners need strong reasoning, broad knowledge
        role_models.insert(AgentRole::Planner, ModelConfig {
            model_id: "gpt-5.2".to_string(),  // Or best reasoning model available
            provider: LlmProvider::OpenAi,
            temperature: 0.7,
            max_tokens: 8192,
            system_prompt_suffix: Some("Focus on strategic planning and task decomposition.".to_string()),
        });

        // Workers need precise code generation
        role_models.insert(AgentRole::Worker, ModelConfig {
            model_id: "claude-opus-4-5-20251101".to_string(),
            provider: LlmProvider::Anthropic,
            temperature: 0.3,
            max_tokens: 16384,
            system_prompt_suffix: Some("Focus on precise implementation and testing.".to_string()),
        });

        // Judges need balanced evaluation
        role_models.insert(AgentRole::Judge, ModelConfig {
            model_id: "claude-sonnet-4-20250514".to_string(),
            provider: LlmProvider::Anthropic,
            temperature: 0.2,
            max_tokens: 4096,
            system_prompt_suffix: Some("Evaluate objectively against criteria.".to_string()),
        });

        Self {
            role_models,
            default_model: ModelConfig {
                model_id: "claude-sonnet-4-20250514".to_string(),
                provider: LlmProvider::Anthropic,
                temperature: 0.5,
                max_tokens: 8192,
                system_prompt_suffix: None,
            },
        }
    }
}
```

---

## 7. Progress Tracking

### Project State

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Project identifier
    pub project_id: ProjectId,
    /// Overall goal
    pub goal: String,
    /// Current phase
    pub phase: ProjectPhase,
    /// Domain states
    pub domains: HashMap<DomainId, DomainState>,
    /// Global metrics
    pub metrics: ProjectMetrics,
    /// Active checkpoints
    pub checkpoints: Vec<CheckpointId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectPhase {
    /// Initial exploration
    Exploration,
    /// Active development
    Development { iteration: u32 },
    /// Refinement and polish
    Refinement,
    /// Final verification
    Verification,
    /// Complete
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    /// Total tasks created
    pub tasks_created: u64,
    /// Tasks completed successfully
    pub tasks_completed: u64,
    /// Tasks failed
    pub tasks_failed: u64,
    /// Lines of code added
    pub lines_added: u64,
    /// Lines of code removed
    pub lines_removed: u64,
    /// Test pass rate
    pub test_pass_rate: f64,
    /// Average task duration
    pub avg_task_duration: Duration,
    /// Active workers count
    pub active_workers: u32,
}
```

### Health Monitoring

```rust
pub struct HealthMonitor {
    /// Project state reference
    project_state: Arc<RwLock<ProjectState>>,
    /// Alert thresholds
    thresholds: HealthThresholds,
    /// Alert channel
    alert_tx: mpsc::Sender<HealthAlert>,
}

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Max consecutive failures before alert
    pub max_consecutive_failures: u32,
    /// Max task duration before considered stuck
    pub max_task_duration: Duration,
    /// Min progress rate (tasks/hour)
    pub min_progress_rate: f64,
    /// Max idle planners
    pub max_idle_planners: u32,
}

#[derive(Debug, Clone)]
pub enum HealthAlert {
    /// Worker appears stuck
    WorkerStuck { worker_id: WorkerId, task_id: TaskId, duration: Duration },
    /// High failure rate
    HighFailureRate { domain_id: DomainId, rate: f64 },
    /// No progress being made
    NoProgress { duration: Duration },
    /// Planner not responding
    PlannerUnresponsive { planner_id: PlannerId },
    /// Resource exhaustion
    ResourceExhaustion { resource: String },
}

impl HealthMonitor {
    /// Periodic health check
    pub async fn check(&self) -> Result<HealthStatus> {
        let state = self.project_state.read().await;
        let mut alerts = Vec::new();

        // Check for stuck workers
        for (domain_id, domain_state) in &state.domains {
            for (worker_id, task_info) in &domain_state.active_workers {
                if task_info.duration() > self.thresholds.max_task_duration {
                    alerts.push(HealthAlert::WorkerStuck {
                        worker_id: worker_id.clone(),
                        task_id: task_info.task_id.clone(),
                        duration: task_info.duration(),
                    });
                }
            }
        }

        // Check progress rate
        let progress_rate = self.calculate_progress_rate(&state);
        if progress_rate < self.thresholds.min_progress_rate {
            alerts.push(HealthAlert::NoProgress {
                duration: self.time_since_last_progress(&state),
            });
        }

        // Send alerts
        for alert in &alerts {
            self.alert_tx.send(alert.clone()).await?;
        }

        Ok(HealthStatus {
            healthy: alerts.is_empty(),
            alerts,
            metrics: state.metrics.clone(),
        })
    }
}
```

---

## 8. Database Schema

### Tables

```sql
-- Project runs
CREATE TABLE project_runs (
    id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL,
    goal TEXT NOT NULL,
    phase TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    status TEXT NOT NULL,
    metrics TEXT  -- JSON
);

-- Domains
CREATE TABLE domains (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES project_runs(id),
    name TEXT NOT NULL,
    file_patterns TEXT NOT NULL,  -- JSON array
    created_at TEXT NOT NULL
);

-- Planners
CREATE TABLE planners (
    id TEXT PRIMARY KEY,
    domain_id TEXT NOT NULL REFERENCES domains(id),
    parent_planner_id TEXT REFERENCES planners(id),
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_active_at TEXT
);

-- Tasks
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    parent_id TEXT REFERENCES tasks(id),
    planner_id TEXT NOT NULL REFERENCES planners(id),
    priority INTEGER NOT NULL,
    task_type TEXT NOT NULL,
    description TEXT NOT NULL,
    specification TEXT NOT NULL,  -- JSON
    affected_files TEXT NOT NULL,  -- JSON array
    status TEXT NOT NULL,
    iteration INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Task dependencies
CREATE TABLE task_dependencies (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    depends_on_task_id TEXT NOT NULL REFERENCES tasks(id),
    PRIMARY KEY (task_id, depends_on_task_id)
);

-- Workers
CREATE TABLE workers (
    id TEXT PRIMARY KEY,
    domain_id TEXT NOT NULL REFERENCES domains(id),
    current_task_id TEXT REFERENCES tasks(id),
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_active_at TEXT
);

-- Task completions
CREATE TABLE task_completions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    worker_id TEXT NOT NULL REFERENCES workers(id),
    completed_at TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    output TEXT NOT NULL,  -- JSON
    changes TEXT NOT NULL,  -- JSON array
    commit_sha TEXT,
    verifications TEXT NOT NULL  -- JSON array
);

-- Checkpoints
CREATE TABLE checkpoints (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES project_runs(id),
    created_at TEXT NOT NULL,
    state_snapshot TEXT NOT NULL,  -- JSON
    reason TEXT
);

-- Learnings (for context preservation)
CREATE TABLE learnings (
    id TEXT PRIMARY KEY,
    project_run_id TEXT NOT NULL REFERENCES project_runs(id),
    domain_id TEXT REFERENCES domains(id),
    learning_type TEXT NOT NULL,
    content TEXT NOT NULL,
    source_task_id TEXT REFERENCES tasks(id),
    created_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_tasks_planner_id ON tasks(planner_id);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_task_completions_task_id ON task_completions(task_id);
CREATE INDEX idx_learnings_domain_id ON learnings(domain_id);
CREATE INDEX idx_workers_domain_id ON workers(domain_id);
```

---

## 9. API Endpoints

### Project Management

```
POST /api/multi-agent/projects
Content-Type: application/json
{
  "project_path": "/path/to/project",
  "goal": "Implement feature X with full test coverage",
  "config": {
    "max_workers": 10,
    "model_config": { ... }
  }
}

Response 201:
{
  "project_run_id": "run-123",
  "status": "initializing",
  "domains": []
}
```

### Status and Control

```
GET /api/multi-agent/projects/{run_id}
GET /api/multi-agent/projects/{run_id}/tasks
GET /api/multi-agent/projects/{run_id}/workers
POST /api/multi-agent/projects/{run_id}/checkpoint
POST /api/multi-agent/projects/{run_id}/pause
POST /api/multi-agent/projects/{run_id}/resume
POST /api/multi-agent/projects/{run_id}/cancel
```

### Real-time Updates (SSE)

```
GET /api/multi-agent/projects/{run_id}/events
Accept: text/event-stream

event: task_completed
data: {"task_id": "task-123", "summary": "Implemented login endpoint"}

event: planner_wake
data: {"planner_id": "planner-456", "reason": "task_completed", "new_tasks": 3}

event: worker_progress
data: {"worker_id": "worker-789", "task_id": "task-124", "progress": 45}
```

---

## 10. Implementation Checklist

### Phase 1: Core Infrastructure

- [ ] Create `loom-multi-agent` crate
- [ ] Implement `Task` and `TaskStatus` types
- [ ] Implement `TaskQueue` with dependency resolution
- [ ] Implement `FileOwnership` conflict prevention
- [ ] Add database migrations for multi-agent tables

### Phase 2: Agent Roles

- [ ] Implement `Orchestrator`
- [ ] Implement `DomainPlanner` with event-driven wake-up
- [ ] Implement `Worker` with task execution loop
- [ ] Implement `JudgeAgent` for completion evaluation
- [ ] Add `ModelSelector` for role-based model selection

### Phase 3: Coordination

- [ ] Implement planner event system (wake-up on task completion)
- [ ] Implement task dependency resolution
- [ ] Add conflict detection and prevention
- [ ] Implement checkpoint/restore for recovery

### Phase 4: Integration

- [ ] Add API endpoints for project management
- [ ] Add SSE endpoint for real-time updates
- [ ] Integrate with existing job scheduler
- [ ] Add admin UI for multi-agent monitoring

### Phase 5: Context Management

- [ ] Integrate with context-management-system.md spec
- [ ] Implement learning absorption from task completions
- [ ] Add context refresh triggers
- [ ] Implement drift detection

---

## 11. Future Enhancements

- Cross-codebase coordination for monorepos
- Human-in-the-loop approval gates for critical changes
- Cost optimization through model routing
- Learning transfer between project runs
- Distributed orchestration for very large codebases
- Integration with external CI/CD systems
