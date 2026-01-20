<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Task Signaling System Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-19

---

## 1. Overview

### Purpose

Provide an event-driven signaling system that enables planners to wake up precisely when their tasks complete, rather than polling or running continuously. This allows planners to immediately plan next steps while task context is fresh.

### Goals

- **Event-driven wake-up**: Planners sleep until relevant events occur
- **Zero-latency signaling**: Immediate notification on task completion
- **Rich completion context**: Include all relevant information for planning next steps
- **Cascade support**: Task completions can trigger dependent task unblocking
- **Back-pressure handling**: Graceful handling when planners are overwhelmed

### Non-Goals

- Real-time collaborative editing signals (different system)
- UI notification system (separate concern)
- External webhook delivery (handled by integration layer)

### Background

From Cursor's research: "Planners should wake up when their tasks complete to plan the next step." This is currently an open problem - planners either poll continuously (wasteful) or miss completion events (suboptimal planning).

---

## 2. Architecture

### Signal Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Event Bus (Singleton)                             │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    Event Router                                      │    │
│  │  - Routes events to subscribed handlers                              │    │
│  │  - Handles back-pressure                                             │    │
│  │  - Maintains subscription registry                                   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │
     ┌──────────────────────────────┼──────────────────────────────┐
     │                              │                              │
     ▼                              ▼                              ▼
┌─────────────┐            ┌─────────────┐            ┌─────────────┐
│   Worker    │            │   Worker    │            │   Worker    │
│             │            │             │            │             │
│ Publishes:  │            │ Publishes:  │            │ Publishes:  │
│ - Progress  │            │ - Progress  │            │ - Progress  │
│ - Complete  │            │ - Complete  │            │ - Complete  │
│ - Failed    │            │ - Failed    │            │ - Failed    │
└─────────────┘            └─────────────┘            └─────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    ▼               ▼               ▼
            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
            │   Planner   │ │   Planner   │ │   Planner   │
            │  (Domain A) │ │  (Domain B) │ │  (Domain C) │
            │             │ │             │ │             │
            │ Subscribes: │ │ Subscribes: │ │ Subscribes: │
            │ - Own tasks │ │ - Own tasks │ │ - Own tasks │
            │ - Dep tasks │ │ - Dep tasks │ │ - Dep tasks │
            └─────────────┘ └─────────────┘ └─────────────┘
```

---

## 3. Event Types

### Task Lifecycle Events

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEvent {
    /// Task was created and queued
    Created(TaskCreatedEvent),
    /// Task was assigned to a worker
    Assigned(TaskAssignedEvent),
    /// Task execution started
    Started(TaskStartedEvent),
    /// Progress update during execution
    Progress(TaskProgressEvent),
    /// Task completed successfully
    Completed(TaskCompletedEvent),
    /// Task failed
    Failed(TaskFailedEvent),
    /// Task was cancelled
    Cancelled(TaskCancelledEvent),
    /// Task requires iteration (judge verdict)
    NeedsIteration(TaskIterationEvent),
    /// Task dependency was resolved
    DependencyResolved(DependencyResolvedEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCreatedEvent {
    /// The task that was created
    pub task: Task,
    /// Planner that created it
    pub planner_id: PlannerId,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignedEvent {
    pub task_id: TaskId,
    pub worker_id: WorkerId,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStartedEvent {
    pub task_id: TaskId,
    pub worker_id: WorkerId,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressEvent {
    pub task_id: TaskId,
    pub worker_id: WorkerId,
    pub progress: TaskProgress,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletedEvent {
    /// Task that completed
    pub task_id: TaskId,
    /// Worker that completed it
    pub worker_id: WorkerId,
    /// Planner that created the task (for routing)
    pub planner_id: PlannerId,
    /// Completion details
    pub completion: TaskCompletion,
    /// Tasks that were unblocked by this completion
    pub unblocked_tasks: Vec<TaskId>,
    /// Suggested follow-up tasks from the worker
    pub suggested_tasks: Vec<TaskSuggestion>,
    /// Learnings extracted from the task
    pub learnings: Vec<Learning>,
    /// Timestamp
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFailedEvent {
    pub task_id: TaskId,
    pub worker_id: WorkerId,
    pub planner_id: PlannerId,
    pub error: TaskError,
    pub recoverable: bool,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancelledEvent {
    pub task_id: TaskId,
    pub reason: CancellationReason,
    pub cancelled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIterationEvent {
    pub task_id: TaskId,
    pub planner_id: PlannerId,
    pub judge_verdict: Verdict,
    pub iteration: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResolvedEvent {
    /// Task that had the dependency
    pub task_id: TaskId,
    /// Dependency that was resolved
    pub dependency_id: TaskId,
    /// Whether all dependencies are now resolved
    pub all_resolved: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}
```

### Supporting Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSuggestion {
    /// Suggested task type
    pub task_type: TaskType,
    /// Description
    pub description: String,
    /// Rationale
    pub rationale: String,
    /// Estimated priority
    pub suggested_priority: i32,
    /// Files that would be affected
    pub affected_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    /// What was learned
    pub insight: String,
    /// Learning category
    pub category: LearningCategory,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
    /// Applicable contexts
    pub applicable_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningCategory {
    /// Code pattern that works well
    CodePattern,
    /// Architecture decision
    ArchitectureDecision,
    /// Error resolution
    ErrorResolution,
    /// Performance optimization
    PerformanceOptimization,
    /// Testing approach
    TestingApproach,
    /// Tool usage
    ToolUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskError {
    /// Error kind
    pub kind: TaskErrorKind,
    /// Error message
    pub message: String,
    /// Stack trace (if available)
    pub stack_trace: Option<String>,
    /// Context at time of failure
    pub context: ErrorContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskErrorKind {
    /// Compilation failed
    CompilationError,
    /// Tests failed
    TestFailure,
    /// Runtime error
    RuntimeError,
    /// Resource exhausted
    ResourceExhausted,
    /// External dependency failed
    ExternalDependency,
    /// Timeout
    Timeout,
    /// Unknown
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CancellationReason {
    /// User requested
    UserRequested,
    /// Superseded by another task
    Superseded { by_task: TaskId },
    /// Conflict detected
    Conflict { with_task: TaskId },
    /// Timeout
    Timeout,
    /// Project shutdown
    ProjectShutdown,
}
```

---

## 4. Event Bus Implementation

### Core Event Bus

```rust
pub struct EventBus {
    /// Event channel capacity
    capacity: usize,
    /// Broadcast sender
    broadcast_tx: broadcast::Sender<TaskEvent>,
    /// Subscription registry
    subscriptions: Arc<RwLock<SubscriptionRegistry>>,
    /// Event persistence (for replay)
    persistence: Arc<EventPersistence>,
    /// Metrics
    metrics: EventBusMetrics,
}

impl EventBus {
    /// Create new event bus
    pub fn new(capacity: usize, persistence: Arc<EventPersistence>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(capacity);

        Self {
            capacity,
            broadcast_tx,
            subscriptions: Arc::new(RwLock::new(SubscriptionRegistry::new())),
            persistence,
            metrics: EventBusMetrics::default(),
        }
    }

    /// Publish an event
    pub async fn publish(&self, event: TaskEvent) -> Result<()> {
        // Persist event for replay
        self.persistence.store(&event).await?;

        // Update metrics
        self.metrics.events_published.fetch_add(1, Ordering::Relaxed);

        // Broadcast to all subscribers
        match self.broadcast_tx.send(event.clone()) {
            Ok(receivers) => {
                self.metrics.events_delivered.fetch_add(receivers as u64, Ordering::Relaxed);
            }
            Err(_) => {
                // No receivers - this is OK, just log
                tracing::debug!("No receivers for event");
            }
        }

        // Also send to specific subscribers based on event content
        self.route_to_specific_subscribers(&event).await?;

        Ok(())
    }

    /// Subscribe to events matching a filter
    pub async fn subscribe(&self, filter: EventFilter) -> EventSubscription {
        let rx = self.broadcast_tx.subscribe();
        let id = SubscriptionId::new();

        let mut registry = self.subscriptions.write().await;
        registry.register(id.clone(), filter.clone());

        EventSubscription {
            id,
            filter,
            rx,
            event_bus: Arc::new(self.clone()),
        }
    }

    /// Route to specific subscribers based on event content
    async fn route_to_specific_subscribers(&self, event: &TaskEvent) -> Result<()> {
        let registry = self.subscriptions.read().await;

        // Route task completion to the planner that created it
        if let TaskEvent::Completed(completed) = event {
            if let Some(channel) = registry.get_planner_channel(&completed.planner_id) {
                channel.send(PlannerEvent::TaskCompleted(completed.completion.clone())).await?;
            }
        }

        // Route iteration feedback to planners
        if let TaskEvent::NeedsIteration(iteration) = event {
            if let Some(channel) = registry.get_planner_channel(&iteration.planner_id) {
                channel.send(PlannerEvent::IterationFeedback {
                    task_id: iteration.task_id.clone(),
                    feedback: iteration.judge_verdict.feedback().to_string(),
                }).await?;
            }
        }

        Ok(())
    }
}
```

### Subscription Management

```rust
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Filter by event types
    pub event_types: Option<HashSet<EventType>>,
    /// Filter by task IDs
    pub task_ids: Option<HashSet<TaskId>>,
    /// Filter by planner IDs
    pub planner_ids: Option<HashSet<PlannerId>>,
    /// Filter by worker IDs
    pub worker_ids: Option<HashSet<WorkerId>>,
    /// Filter by domain IDs
    pub domain_ids: Option<HashSet<DomainId>>,
}

impl EventFilter {
    /// Create a filter for all events from a specific planner
    pub fn for_planner(planner_id: PlannerId) -> Self {
        Self {
            planner_ids: Some(HashSet::from([planner_id])),
            ..Default::default()
        }
    }

    /// Create a filter for completion events only
    pub fn completions_only() -> Self {
        Self {
            event_types: Some(HashSet::from([EventType::Completed])),
            ..Default::default()
        }
    }

    /// Create a filter for a specific task
    pub fn for_task(task_id: TaskId) -> Self {
        Self {
            task_ids: Some(HashSet::from([task_id])),
            ..Default::default()
        }
    }

    /// Check if event matches filter
    pub fn matches(&self, event: &TaskEvent) -> bool {
        // Check event type
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type()) {
                return false;
            }
        }

        // Check task ID
        if let Some(ref task_ids) = self.task_ids {
            if !task_ids.contains(&event.task_id()) {
                return false;
            }
        }

        // Check planner ID
        if let Some(ref planner_ids) = self.planner_ids {
            if let Some(planner_id) = event.planner_id() {
                if !planner_ids.contains(&planner_id) {
                    return false;
                }
            }
        }

        true
    }
}

pub struct EventSubscription {
    /// Subscription ID
    id: SubscriptionId,
    /// Filter
    filter: EventFilter,
    /// Receiver
    rx: broadcast::Receiver<TaskEvent>,
    /// Event bus reference (for unsubscribe)
    event_bus: Arc<EventBus>,
}

impl EventSubscription {
    /// Wait for next matching event
    pub async fn next(&mut self) -> Option<TaskEvent> {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Some(event);
                    }
                    // Event doesn't match filter, continue waiting
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(subscription_id = %self.id, lagged = n, "Subscription lagged");
                    // Continue trying to receive
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return None;
                }
            }
        }
    }

    /// Wait for next event with timeout
    pub async fn next_timeout(&mut self, timeout: Duration) -> Result<TaskEvent, EventError> {
        tokio::time::timeout(timeout, self.next())
            .await
            .map_err(|_| EventError::Timeout)?
            .ok_or(EventError::Closed)
    }
}

impl Drop for EventSubscription {
    fn drop(&mut self) {
        // Unsubscribe when dropped
        // Note: This is best-effort since we can't await in drop
        let event_bus = self.event_bus.clone();
        let id = self.id.clone();
        tokio::spawn(async move {
            let mut registry = event_bus.subscriptions.write().await;
            registry.unregister(&id);
        });
    }
}
```

### Subscription Registry

```rust
pub struct SubscriptionRegistry {
    /// All subscriptions
    subscriptions: HashMap<SubscriptionId, EventFilter>,
    /// Planner-specific channels (for direct routing)
    planner_channels: HashMap<PlannerId, mpsc::Sender<PlannerEvent>>,
    /// Worker-specific channels
    worker_channels: HashMap<WorkerId, mpsc::Sender<WorkerEvent>>,
}

impl SubscriptionRegistry {
    /// Register a subscription
    pub fn register(&mut self, id: SubscriptionId, filter: EventFilter) {
        self.subscriptions.insert(id, filter);
    }

    /// Unregister a subscription
    pub fn unregister(&mut self, id: &SubscriptionId) {
        self.subscriptions.remove(id);
    }

    /// Register planner channel for direct routing
    pub fn register_planner(&mut self, planner_id: PlannerId, tx: mpsc::Sender<PlannerEvent>) {
        self.planner_channels.insert(planner_id, tx);
    }

    /// Get planner channel
    pub fn get_planner_channel(&self, planner_id: &PlannerId) -> Option<&mpsc::Sender<PlannerEvent>> {
        self.planner_channels.get(planner_id)
    }
}
```

---

## 5. Planner Wake-up Implementation

### Planner Event Loop

```rust
impl DomainPlanner {
    /// Main planner loop - sleeps until events arrive
    pub async fn run(&mut self) -> Result<()> {
        // Subscribe to relevant events
        let mut subscription = self.event_bus.subscribe(EventFilter {
            planner_ids: Some(HashSet::from([self.id.clone()])),
            event_types: Some(HashSet::from([
                EventType::Completed,
                EventType::Failed,
                EventType::NeedsIteration,
                EventType::DependencyResolved,
            ])),
            ..Default::default()
        }).await;

        loop {
            // SLEEP - Wait for events (this is the key wake-up mechanism)
            tokio::select! {
                // Direct planner events (higher priority)
                Some(event) = self.event_rx.recv() => {
                    self.handle_planner_event(event).await?;
                }

                // Broadcast events
                Some(event) = subscription.next() => {
                    self.handle_task_event(event).await?;
                }

                // Periodic wake-up for health check (infrequent)
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    self.health_check().await?;
                }
            }
        }
    }

    /// Handle direct planner events
    async fn handle_planner_event(&mut self, event: PlannerEvent) -> Result<()> {
        match event {
            PlannerEvent::TaskCompleted(completion) => {
                tracing::info!(
                    planner_id = %self.id,
                    task_id = %completion.task_id,
                    "Planner woke up: task completed"
                );

                // Immediately plan next steps while context is fresh
                let new_tasks = self.plan_next_steps(&completion).await?;

                // Submit new tasks
                for task in new_tasks {
                    self.task_tx.send(task).await?;
                }

                // Update metrics
                self.metrics.wake_ups.fetch_add(1, Ordering::Relaxed);
            }

            PlannerEvent::IterationFeedback { task_id, feedback } => {
                tracing::info!(
                    planner_id = %self.id,
                    task_id = %task_id,
                    "Planner woke up: iteration needed"
                );

                // Create iteration task with judge feedback
                let iteration_task = self.create_iteration_task(&task_id, &feedback).await?;
                self.task_tx.send(iteration_task).await?;
            }

            PlannerEvent::DomainChanged { changes, .. } => {
                tracing::info!(
                    planner_id = %self.id,
                    num_changes = changes.len(),
                    "Planner woke up: domain changed"
                );

                // Re-evaluate plans based on changes
                let reactive_tasks = self.react_to_changes(&changes).await?;
                for task in reactive_tasks {
                    self.task_tx.send(task).await?;
                }
            }

            PlannerEvent::Shutdown => {
                tracing::info!(planner_id = %self.id, "Planner shutting down");
                return Ok(());
            }

            _ => {}
        }

        Ok(())
    }

    /// Plan next steps after task completion
    async fn plan_next_steps(&mut self, completion: &TaskCompletion) -> Result<Vec<Task>> {
        let mut next_tasks = Vec::new();

        // 1. Absorb learnings from completion
        self.absorb_learnings(completion).await?;

        // 2. Process worker suggestions
        for suggestion in &completion.output.suggested_tasks {
            if self.should_accept_suggestion(suggestion).await? {
                let task = self.create_task_from_suggestion(suggestion).await?;
                next_tasks.push(task);
            }
        }

        // 3. Check if this completion enables larger goals
        let enabled_goals = self.check_enabled_goals(completion).await?;
        for goal in enabled_goals {
            let tasks = self.decompose_goal(&goal).await?;
            next_tasks.extend(tasks);
        }

        // 4. Analyze changes for follow-up needs
        let follow_ups = self.analyze_changes_for_follow_ups(&completion.changes).await?;
        next_tasks.extend(follow_ups);

        // 5. Update domain model
        self.update_domain_model(completion).await?;

        tracing::info!(
            planner_id = %self.id,
            task_id = %completion.task_id,
            new_tasks = next_tasks.len(),
            "Planned next steps"
        );

        Ok(next_tasks)
    }

    /// Absorb learnings into planner's context
    async fn absorb_learnings(&mut self, completion: &TaskCompletion) -> Result<()> {
        for learning in &completion.output.learnings {
            // Add to memory store
            let memory_item = MemoryItem {
                id: MemoryId::new(),
                memory_type: MemoryType::LongTerm,  // Learnings are valuable
                content: MemoryContent::InteractionLearning {
                    situation: completion.output.summary.clone(),
                    approach: format!("Task: {}", completion.task_id),
                    outcome: "Completed successfully".to_string(),
                    lesson: learning.insight.clone(),
                },
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                relevance_score: learning.confidence,
                tags: learning.applicable_to.clone(),
                source: MemorySource::TaskCompletion(completion.task_id.clone()),
            };

            self.context_manager.memory.write().await.add(memory_item).await?;
        }

        Ok(())
    }
}
```

### Wake-up Context Preservation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeUpContext {
    /// The event that triggered wake-up
    pub trigger_event: TaskEvent,
    /// Time since planner last planned
    pub time_since_last_plan: Duration,
    /// Number of tasks completed since last planning
    pub tasks_completed_since_plan: u32,
    /// Current domain state snapshot
    pub domain_snapshot: DomainSnapshot,
    /// Pending tasks from this planner
    pub pending_tasks: Vec<TaskId>,
    /// Resource usage at wake-up
    pub resource_usage: ResourceUsage,
}

impl DomainPlanner {
    /// Capture context when waking up
    fn capture_wake_up_context(&self, trigger: &TaskEvent) -> WakeUpContext {
        WakeUpContext {
            trigger_event: trigger.clone(),
            time_since_last_plan: self.last_plan_time.elapsed(),
            tasks_completed_since_plan: self.tasks_since_plan.load(Ordering::Relaxed),
            domain_snapshot: self.domain_model.snapshot(),
            pending_tasks: self.get_pending_task_ids(),
            resource_usage: self.context_manager.current_usage(),
        }
    }

    /// Include wake-up context in planning prompt
    fn build_planning_prompt(&self, context: &WakeUpContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("## Wake-Up Context\n\n");
        prompt.push_str(&format!(
            "You are being activated because: {}\n",
            self.describe_trigger(&context.trigger_event)
        ));
        prompt.push_str(&format!(
            "Time since last planning: {:?}\n",
            context.time_since_last_plan
        ));
        prompt.push_str(&format!(
            "Tasks completed since then: {}\n\n",
            context.tasks_completed_since_plan
        ));

        // Include completion details if this is a task completion
        if let TaskEvent::Completed(completed) = &context.trigger_event {
            prompt.push_str("## Just Completed Task\n\n");
            prompt.push_str(&format!("Summary: {}\n", completed.completion.output.summary));
            prompt.push_str(&format!("Files changed: {:?}\n", completed.completion.changes));

            if !completed.suggested_tasks.is_empty() {
                prompt.push_str("\nWorker suggestions:\n");
                for suggestion in &completed.suggested_tasks {
                    prompt.push_str(&format!("- {}: {}\n", suggestion.description, suggestion.rationale));
                }
            }

            if !completed.learnings.is_empty() {
                prompt.push_str("\nLearnings:\n");
                for learning in &completed.learnings {
                    prompt.push_str(&format!("- {}\n", learning.insight));
                }
            }
        }

        prompt.push_str("\n## Current Domain State\n\n");
        prompt.push_str(&self.describe_domain_state(&context.domain_snapshot));

        prompt.push_str("\n## Pending Tasks\n\n");
        if context.pending_tasks.is_empty() {
            prompt.push_str("No pending tasks.\n");
        } else {
            for task_id in &context.pending_tasks {
                if let Some(task) = self.get_task(task_id) {
                    prompt.push_str(&format!("- {}: {}\n", task_id, task.description));
                }
            }
        }

        prompt
    }
}
```

---

## 6. Cascade Handling

### Dependency Resolution Events

```rust
impl TaskQueue {
    /// Handle task completion and trigger cascades
    pub async fn handle_completion(&mut self, completion: TaskCompletion) -> Result<CascadeResult> {
        let task_id = &completion.task_id;
        let mut cascade_result = CascadeResult::default();

        // Find tasks blocked by this one
        if let Some(blocked_tasks) = self.blocked_by.remove(task_id) {
            for blocked_id in blocked_tasks {
                if let Some(blocked_task) = self.tasks.get_mut(&blocked_id) {
                    // Remove this dependency
                    blocked_task.dependencies.retain(|d| d != task_id);

                    let all_resolved = blocked_task.dependencies.is_empty();

                    // Publish dependency resolved event
                    self.event_bus.publish(TaskEvent::DependencyResolved(
                        DependencyResolvedEvent {
                            task_id: blocked_id.clone(),
                            dependency_id: task_id.clone(),
                            all_resolved,
                            timestamp: Utc::now(),
                        }
                    )).await?;

                    // If all dependencies resolved, move to ready queue
                    if all_resolved {
                        blocked_task.status = TaskStatus::Pending;
                        self.ready.push(PrioritizedTask::from(blocked_task));
                        cascade_result.unblocked.push(blocked_id.clone());
                    }
                }
            }
        }

        // Publish completion event (this wakes up the planner)
        let completed_event = TaskCompletedEvent {
            task_id: task_id.clone(),
            worker_id: completion.worker_id.clone(),
            planner_id: self.tasks.get(task_id)
                .map(|t| t.created_by.clone())
                .unwrap_or_default(),
            completion,
            unblocked_tasks: cascade_result.unblocked.clone(),
            suggested_tasks: Vec::new(),  // Filled by worker
            learnings: Vec::new(),  // Filled by worker
            completed_at: Utc::now(),
        };

        self.event_bus.publish(TaskEvent::Completed(completed_event)).await?;

        Ok(cascade_result)
    }
}

#[derive(Debug, Default)]
pub struct CascadeResult {
    /// Tasks that were unblocked
    pub unblocked: Vec<TaskId>,
    /// Planners that were notified
    pub planners_notified: Vec<PlannerId>,
}
```

### Cascade Batching

```rust
pub struct CascadeBatcher {
    /// Pending cascades
    pending: HashMap<PlannerId, Vec<TaskEvent>>,
    /// Batch timeout
    batch_timeout: Duration,
    /// Maximum batch size
    max_batch_size: usize,
}

impl CascadeBatcher {
    /// Add event to batch
    pub fn add(&mut self, event: TaskEvent) {
        let planner_id = event.planner_id().unwrap_or_default();
        self.pending.entry(planner_id).or_default().push(event);
    }

    /// Flush batches that are ready
    pub async fn flush(&mut self, event_bus: &EventBus) -> Result<()> {
        for (planner_id, events) in self.pending.drain() {
            if events.len() >= self.max_batch_size {
                // Send as batch
                event_bus.publish(TaskEvent::Batch(TaskBatchEvent {
                    planner_id,
                    events,
                    timestamp: Utc::now(),
                })).await?;
            } else {
                // Send individually
                for event in events {
                    event_bus.publish(event).await?;
                }
            }
        }
        Ok(())
    }
}
```

---

## 7. Back-pressure Handling

### Planner Overload Detection

```rust
pub struct BackpressureController {
    /// Queue depth thresholds
    thresholds: BackpressureThresholds,
    /// Current queue depths per planner
    queue_depths: HashMap<PlannerId, usize>,
}

#[derive(Debug, Clone)]
pub struct BackpressureThresholds {
    /// Warning threshold
    pub warning: usize,
    /// Critical threshold (start dropping low-priority)
    pub critical: usize,
    /// Maximum (reject new events)
    pub maximum: usize,
}

impl Default for BackpressureThresholds {
    fn default() -> Self {
        Self {
            warning: 100,
            critical: 500,
            maximum: 1000,
        }
    }
}

impl BackpressureController {
    /// Check if planner can accept more events
    pub fn can_accept(&self, planner_id: &PlannerId, event: &TaskEvent) -> BackpressureDecision {
        let depth = self.queue_depths.get(planner_id).copied().unwrap_or(0);

        if depth >= self.thresholds.maximum {
            return BackpressureDecision::Reject;
        }

        if depth >= self.thresholds.critical {
            // Only accept high-priority events
            if event.priority() < EventPriority::High {
                return BackpressureDecision::Defer;
            }
        }

        if depth >= self.thresholds.warning {
            return BackpressureDecision::AcceptWithWarning;
        }

        BackpressureDecision::Accept
    }

    /// Handle back-pressure decision
    pub async fn handle_backpressure(
        &mut self,
        planner_id: &PlannerId,
        event: TaskEvent,
        decision: BackpressureDecision,
    ) -> Result<()> {
        match decision {
            BackpressureDecision::Accept | BackpressureDecision::AcceptWithWarning => {
                // Normal delivery
                *self.queue_depths.entry(planner_id.clone()).or_default() += 1;
            }
            BackpressureDecision::Defer => {
                // Queue for later delivery
                self.defer_event(planner_id, event).await?;
            }
            BackpressureDecision::Reject => {
                // Log and drop
                tracing::error!(
                    planner_id = %planner_id,
                    event_type = ?event.event_type(),
                    "Event rejected due to back-pressure"
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BackpressureDecision {
    Accept,
    AcceptWithWarning,
    Defer,
    Reject,
}
```

---

## 8. Event Persistence and Replay

### Event Log

```rust
pub struct EventPersistence {
    /// Database pool
    db: SqlitePool,
    /// Write buffer
    buffer: RwLock<Vec<TaskEvent>>,
    /// Buffer flush threshold
    flush_threshold: usize,
}

impl EventPersistence {
    /// Store event
    pub async fn store(&self, event: &TaskEvent) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        buffer.push(event.clone());

        if buffer.len() >= self.flush_threshold {
            self.flush(&mut buffer).await?;
        }

        Ok(())
    }

    /// Flush buffer to database
    async fn flush(&self, buffer: &mut Vec<TaskEvent>) -> Result<()> {
        let events: Vec<_> = buffer.drain(..).collect();

        for event in events {
            sqlx::query(
                "INSERT INTO task_events (id, event_type, payload, timestamp)
                 VALUES (?, ?, ?, ?)"
            )
            .bind(Uuid::new_v4().to_string())
            .bind(event.event_type().as_str())
            .bind(serde_json::to_string(&event)?)
            .bind(event.timestamp().to_rfc3339())
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    /// Replay events from a point in time
    pub async fn replay_from(&self, timestamp: DateTime<Utc>) -> Result<Vec<TaskEvent>> {
        let rows = sqlx::query(
            "SELECT payload FROM task_events
             WHERE timestamp >= ?
             ORDER BY timestamp ASC"
        )
        .bind(timestamp.to_rfc3339())
        .fetch_all(&self.db)
        .await?;

        let events: Vec<TaskEvent> = rows.iter()
            .filter_map(|row| {
                let payload: String = row.get("payload");
                serde_json::from_str(&payload).ok()
            })
            .collect();

        Ok(events)
    }

    /// Replay events for a specific planner (for recovery)
    pub async fn replay_for_planner(
        &self,
        planner_id: &PlannerId,
        since: DateTime<Utc>,
    ) -> Result<Vec<TaskEvent>> {
        // This would use an index on planner_id
        let all_events = self.replay_from(since).await?;

        Ok(all_events.into_iter()
            .filter(|e| e.planner_id() == Some(planner_id.clone()))
            .collect())
    }
}
```

---

## 9. Database Schema

```sql
-- Task events log
CREATE TABLE task_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    task_id TEXT,
    planner_id TEXT,
    worker_id TEXT,
    payload TEXT NOT NULL,  -- JSON
    timestamp TEXT NOT NULL,
    processed INTEGER NOT NULL DEFAULT 0
);

-- Index for replay queries
CREATE INDEX idx_task_events_timestamp ON task_events(timestamp);
CREATE INDEX idx_task_events_planner ON task_events(planner_id, timestamp);
CREATE INDEX idx_task_events_task ON task_events(task_id);
CREATE INDEX idx_task_events_unprocessed ON task_events(processed) WHERE processed = 0;

-- Subscription state (for persistent subscriptions)
CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    subscriber_type TEXT NOT NULL,  -- planner, worker, external
    subscriber_id TEXT NOT NULL,
    filter TEXT NOT NULL,  -- JSON
    last_event_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Deferred events (for back-pressure)
CREATE TABLE deferred_events (
    id TEXT PRIMARY KEY,
    planner_id TEXT NOT NULL,
    event_payload TEXT NOT NULL,  -- JSON
    deferred_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT NOT NULL
);

CREATE INDEX idx_deferred_next_retry ON deferred_events(next_retry_at);
```

---

## 10. API Endpoints

### Event Stream (SSE)

```
GET /api/multi-agent/projects/{run_id}/events
Accept: text/event-stream
Authorization: Bearer {token}

Query Parameters:
- filter: JSON-encoded EventFilter
- since: ISO 8601 timestamp (replay from this point)

Response (SSE):
event: task_completed
id: evt-123
data: {"task_id": "task-456", "planner_id": "planner-789", ...}

event: planner_wake
id: evt-124
data: {"planner_id": "planner-789", "reason": "task_completed", "new_tasks": 3}
```

### Event History

```
GET /api/multi-agent/projects/{run_id}/events/history
Authorization: Bearer {token}

Query Parameters:
- since: ISO 8601 timestamp
- until: ISO 8601 timestamp
- event_types: comma-separated list
- planner_id: filter by planner
- limit: max results (default 100)

Response 200:
{
  "events": [...],
  "total": 1234,
  "has_more": true
}
```

### Subscription Management

```
POST /api/multi-agent/subscriptions
Authorization: Bearer {token}
Content-Type: application/json

{
  "filter": {
    "event_types": ["completed", "failed"],
    "planner_ids": ["planner-123"]
  },
  "webhook_url": "https://example.com/webhook"  // Optional
}

Response 201:
{
  "subscription_id": "sub-456",
  "filter": {...}
}
```

---

## 11. Implementation Checklist

### Phase 1: Core Event System

- [ ] Create `loom-events` crate
- [ ] Implement `TaskEvent` types
- [ ] Implement `EventBus` with broadcast
- [ ] Add `EventFilter` and subscriptions
- [ ] Add database migrations for events

### Phase 2: Planner Integration

- [ ] Implement planner event loop with `select!`
- [ ] Add wake-up context capture
- [ ] Implement `plan_next_steps` logic
- [ ] Add learning absorption

### Phase 3: Cascade Handling

- [ ] Implement dependency resolution events
- [ ] Add cascade batching
- [ ] Integrate with task queue

### Phase 4: Reliability

- [ ] Implement event persistence
- [ ] Add replay functionality
- [ ] Implement back-pressure controller
- [ ] Add deferred event handling

### Phase 5: Integration

- [ ] Add SSE endpoint for events
- [ ] Add history API
- [ ] Add webhook delivery
- [ ] Integrate with admin UI

---

## 12. Future Enhancements

- Event sourcing for full system replay
- Cross-project event federation
- Machine learning for event priority prediction
- Automatic subscription optimization
- Event schema versioning
- Dead letter queue for failed deliveries
