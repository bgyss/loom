<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Context Management System Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-19

---

## 1. Overview

### Purpose

Provide strategies to maintain agent context integrity over extended operations (hours to weeks), avoiding the need for periodic "fresh starts" that discard accumulated context and knowledge.

### Goals

- **Drift prevention**: Detect and correct context drift before it causes problems
- **Bounded resources**: Explicit resource awareness to prevent context bloat
- **Verifiable feedback**: Ground reasoning in executable reality rather than drifting interpretations
- **Knowledge preservation**: Retain valuable learnings while discarding noise
- **Continuous refinement**: Improve context quality over time without full resets

### Non-Goals

- Unlimited context windows (work within bounded resources)
- Perfect memory (focus on relevant, actionable context)
- Zero information loss (strategic forgetting is valuable)

### Background

Research from CodeAdapt demonstrates:

1. **Bounded resource awareness**: Models that reason about their resource constraints (time, tokens, steps) perform better than those that don't
2. **Verifiable feedback loops**: Grounding reasoning in executable code/tests prevents ambiguous state progression
3. **Persistent state across turns**: Maintaining state enables iterative refinement without context loss
4. **Dynamic strategy selection**: Adapting approach based on intermediate results prevents tunnel vision

---

## 2. Architecture

### Context Management Pipeline

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         Context Management System                             │
└──────────────────────────────────────────────────────────────────────────────┘
                                      │
         ┌────────────────────────────┼────────────────────────────┐
         ▼                            ▼                            ▼
┌──────────────────┐      ┌──────────────────┐      ┌──────────────────┐
│  Context Intake  │      │  Context Store   │      │  Context Export  │
│                  │      │                  │      │                  │
│  - Event capture │      │  - Structured    │      │  - Window        │
│  - Relevance     │      │    memory        │      │    assembly      │
│    scoring       │      │  - Decay/refresh │      │  - Compression   │
│  - Deduplication │      │  - Indexing      │      │  - Prioritization│
└────────┬─────────┘      └────────┬─────────┘      └────────┬─────────┘
         │                         │                         │
         └─────────────────────────┼─────────────────────────┘
                                   ▼
                    ┌──────────────────────────┐
                    │    Drift Detector        │
                    │                          │
                    │  - Consistency checks    │
                    │  - Reality grounding     │
                    │  - Pattern detection     │
                    └────────────┬─────────────┘
                                 │
                    ┌────────────┼────────────┐
                    ▼            ▼            ▼
             ┌──────────┐ ┌──────────┐ ┌──────────┐
             │ Correct  │ │ Refresh  │ │ Alert    │
             │ Drift    │ │ Context  │ │ Operator │
             └──────────┘ └──────────┘ └──────────┘
```

---

## 3. Bounded Resource Model

### Resource Constraints

Explicit limits that agents reason about and respect.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    /// Maximum tokens per context window
    pub max_context_tokens: u32,
    /// Maximum tokens per single operation
    pub max_operation_tokens: u32,
    /// Maximum time per task
    pub max_task_duration: Duration,
    /// Maximum reasoning steps per task
    pub max_reasoning_steps: u32,
    /// Maximum retries before escalation
    pub max_retries: u32,
    /// Memory budget for persistent state
    pub max_memory_items: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_context_tokens: 128_000,  // ~100K effective with overhead
            max_operation_tokens: 16_000,
            max_task_duration: Duration::from_secs(1800),  // 30 minutes
            max_reasoning_steps: 25,
            max_retries: 3,
            max_memory_items: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// Current context token usage
    pub context_tokens: u32,
    /// Tokens used in current operation
    pub operation_tokens: u32,
    /// Time elapsed on current task
    pub task_elapsed: Duration,
    /// Reasoning steps taken
    pub reasoning_steps: u32,
    /// Current memory item count
    pub memory_items: usize,
}

impl ResourceUsage {
    /// Check if near budget limits
    pub fn is_near_limit(&self, budget: &ResourceBudget, threshold: f32) -> ResourceWarnings {
        let mut warnings = ResourceWarnings::default();

        if self.context_tokens as f32 > budget.max_context_tokens as f32 * threshold {
            warnings.context_pressure = true;
        }
        if self.task_elapsed > budget.max_task_duration.mul_f32(threshold) {
            warnings.time_pressure = true;
        }
        if self.reasoning_steps as f32 > budget.max_reasoning_steps as f32 * threshold {
            warnings.step_pressure = true;
        }

        warnings
    }

    /// Calculate remaining budget
    pub fn remaining(&self, budget: &ResourceBudget) -> ResourceRemaining {
        ResourceRemaining {
            context_tokens: budget.max_context_tokens.saturating_sub(self.context_tokens),
            task_time: budget.max_task_duration.saturating_sub(self.task_elapsed),
            reasoning_steps: budget.max_reasoning_steps.saturating_sub(self.reasoning_steps),
        }
    }
}
```

### Resource-Aware Prompting

Include resource state in prompts so agents can adapt strategies.

```rust
pub struct ResourceAwarePromptBuilder {
    budget: ResourceBudget,
    usage: ResourceUsage,
}

impl ResourceAwarePromptBuilder {
    /// Build resource awareness section for prompts
    pub fn build_resource_section(&self) -> String {
        let remaining = self.usage.remaining(&self.budget);
        let warnings = self.usage.is_near_limit(&self.budget, 0.8);

        let mut section = String::new();
        section.push_str("## Resource Status\n\n");

        section.push_str(&format!(
            "- Context budget: {}/{} tokens ({:.0}% used)\n",
            self.usage.context_tokens,
            self.budget.max_context_tokens,
            (self.usage.context_tokens as f64 / self.budget.max_context_tokens as f64) * 100.0
        ));

        section.push_str(&format!(
            "- Time remaining: {:?} of {:?}\n",
            remaining.task_time,
            self.budget.max_task_duration
        ));

        section.push_str(&format!(
            "- Reasoning steps: {}/{}\n",
            self.usage.reasoning_steps,
            self.budget.max_reasoning_steps
        ));

        if warnings.any() {
            section.push_str("\n**Resource Warnings:**\n");
            if warnings.context_pressure {
                section.push_str("- Context window near capacity. Consider summarizing or completing soon.\n");
            }
            if warnings.time_pressure {
                section.push_str("- Task time limit approaching. Prioritize completion.\n");
            }
            if warnings.step_pressure {
                section.push_str("- Reasoning step limit approaching. Converge on solution.\n");
            }
        }

        section
    }
}
```

---

## 4. Structured Memory System

### Memory Types

Different memory types with different retention and access patterns.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// Immediate working memory (current task)
    Working,
    /// Short-term memory (recent tasks, decays)
    ShortTerm,
    /// Long-term memory (important learnings, persists)
    LongTerm,
    /// Episodic memory (specific events, searchable)
    Episodic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique identifier
    pub id: MemoryId,
    /// Memory type
    pub memory_type: MemoryType,
    /// Content
    pub content: MemoryContent,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Access count
    pub access_count: u32,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f64,
    /// Tags for retrieval
    pub tags: Vec<String>,
    /// Source (task, file, conversation)
    pub source: MemorySource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryContent {
    /// Code pattern or solution
    CodePattern {
        pattern: String,
        example: String,
        applicability: String,
    },
    /// Architectural decision
    Decision {
        context: String,
        decision: String,
        rationale: String,
        alternatives_considered: Vec<String>,
    },
    /// Error and resolution
    ErrorResolution {
        error_pattern: String,
        root_cause: String,
        solution: String,
    },
    /// File/codebase knowledge
    CodebaseKnowledge {
        path: PathBuf,
        summary: String,
        key_concepts: Vec<String>,
    },
    /// Constraint or requirement
    Constraint {
        description: String,
        source: String,
        implications: Vec<String>,
    },
    /// Interaction learning
    InteractionLearning {
        situation: String,
        approach: String,
        outcome: String,
        lesson: String,
    },
}
```

### Memory Store

```rust
pub struct MemoryStore {
    /// All memory items
    items: HashMap<MemoryId, MemoryItem>,
    /// Index by type
    by_type: HashMap<MemoryType, HashSet<MemoryId>>,
    /// Index by tag
    by_tag: HashMap<String, HashSet<MemoryId>>,
    /// Full-text search index
    search_index: SearchIndex,
    /// Decay configuration
    decay_config: DecayConfig,
    /// Budget
    budget: ResourceBudget,
}

#[derive(Debug, Clone)]
pub struct DecayConfig {
    /// Short-term memory half-life
    pub short_term_half_life: Duration,
    /// Minimum relevance before pruning
    pub min_relevance: f64,
    /// Access boost factor
    pub access_boost: f64,
}

impl MemoryStore {
    /// Add a memory item
    pub async fn add(&mut self, item: MemoryItem) -> Result<MemoryId> {
        // Check budget
        if self.items.len() >= self.budget.max_memory_items {
            // Prune lowest relevance items
            self.prune_to_budget().await?;
        }

        let id = item.id.clone();
        self.index_item(&item);
        self.items.insert(id.clone(), item);
        Ok(id)
    }

    /// Retrieve relevant memories for a query
    pub async fn retrieve(&mut self, query: &MemoryQuery) -> Vec<&MemoryItem> {
        let mut results: Vec<_> = self.items.values()
            .filter(|item| self.matches_query(item, query))
            .collect();

        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        // Update access timestamps
        for item in &results {
            if let Some(item) = self.items.get_mut(&item.id) {
                item.last_accessed = Utc::now();
                item.access_count += 1;
                item.relevance_score = (item.relevance_score + self.decay_config.access_boost).min(1.0);
            }
        }

        // Limit results
        results.truncate(query.max_results.unwrap_or(10));
        results
    }

    /// Apply decay to short-term memories
    pub async fn apply_decay(&mut self) {
        let now = Utc::now();

        for item in self.items.values_mut() {
            if item.memory_type == MemoryType::ShortTerm {
                let age = now - item.last_accessed;
                let decay_factor = 0.5_f64.powf(
                    age.num_seconds() as f64 / self.decay_config.short_term_half_life.as_secs() as f64
                );
                item.relevance_score *= decay_factor;
            }
        }

        // Prune items below threshold
        self.items.retain(|_, item| {
            item.memory_type == MemoryType::LongTerm ||
            item.relevance_score >= self.decay_config.min_relevance
        });
    }

    /// Promote short-term memory to long-term
    pub async fn promote(&mut self, id: &MemoryId) -> Result<()> {
        if let Some(item) = self.items.get_mut(id) {
            item.memory_type = MemoryType::LongTerm;
            item.relevance_score = 1.0;  // Reset relevance
        }
        Ok(())
    }

    /// Consolidate similar memories
    pub async fn consolidate(&mut self) -> Result<u32> {
        let mut consolidated = 0;

        // Group similar items
        let groups = self.find_similar_groups().await?;

        for group in groups {
            if group.len() > 1 {
                // Merge into single consolidated memory
                let merged = self.merge_memories(&group)?;

                // Remove originals
                for id in &group {
                    self.items.remove(id);
                }

                // Add merged
                self.items.insert(merged.id.clone(), merged);
                consolidated += group.len() as u32 - 1;
            }
        }

        Ok(consolidated)
    }
}
```

---

## 5. Verifiable Feedback Loops

### Grounding in Executable Reality

```rust
pub struct VerificationEngine {
    /// Available verification methods
    verifiers: HashMap<String, Box<dyn Verifier>>,
    /// Verification history
    history: Vec<VerificationResult>,
}

#[async_trait]
pub trait Verifier: Send + Sync {
    /// Verify a claim
    async fn verify(&self, claim: &Claim) -> Result<VerificationResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// What is being claimed
    pub assertion: String,
    /// Type of claim
    pub claim_type: ClaimType,
    /// Evidence/context
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimType {
    /// Code compiles
    Compiles { files: Vec<PathBuf> },
    /// Tests pass
    TestsPass { test_pattern: String },
    /// File contains pattern
    FileContains { path: PathBuf, pattern: String },
    /// API returns expected result
    ApiReturns { endpoint: String, expected_status: u16 },
    /// Command succeeds
    CommandSucceeds { command: String },
    /// Type checks
    TypeChecks { files: Vec<PathBuf> },
    /// Lint passes
    LintPasses { files: Vec<PathBuf> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Claim that was verified
    pub claim: Claim,
    /// Whether verification passed
    pub passed: bool,
    /// Output/evidence
    pub output: String,
    /// Timestamp
    pub verified_at: DateTime<Utc>,
    /// Duration
    pub duration: Duration,
}

impl VerificationEngine {
    /// Verify all claims and ground reasoning
    pub async fn verify_claims(&mut self, claims: &[Claim]) -> Vec<VerificationResult> {
        let mut results = Vec::new();

        for claim in claims {
            let verifier_name = self.verifier_for_claim(&claim.claim_type);
            if let Some(verifier) = self.verifiers.get(&verifier_name) {
                match verifier.verify(claim).await {
                    Ok(result) => {
                        self.history.push(result.clone());
                        results.push(result);
                    }
                    Err(e) => {
                        results.push(VerificationResult {
                            claim: claim.clone(),
                            passed: false,
                            output: format!("Verification failed: {}", e),
                            verified_at: Utc::now(),
                            duration: Duration::ZERO,
                        });
                    }
                }
            }
        }

        results
    }

    /// Check if recent verifications show drift
    pub fn detect_drift(&self, window: Duration) -> Option<DriftSignal> {
        let cutoff = Utc::now() - chrono::Duration::from_std(window).unwrap();
        let recent: Vec<_> = self.history.iter()
            .filter(|r| r.verified_at > cutoff)
            .collect();

        if recent.is_empty() {
            return None;
        }

        let failure_rate = recent.iter().filter(|r| !r.passed).count() as f64 / recent.len() as f64;

        if failure_rate > 0.5 {
            return Some(DriftSignal::HighFailureRate { rate: failure_rate });
        }

        // Check for repeated failures on same claim type
        let mut failure_counts: HashMap<&str, u32> = HashMap::new();
        for result in recent.iter().filter(|r| !r.passed) {
            let key = self.claim_type_key(&result.claim.claim_type);
            *failure_counts.entry(key).or_default() += 1;
        }

        for (claim_type, count) in failure_counts {
            if count > 3 {
                return Some(DriftSignal::RepeatedFailures {
                    claim_type: claim_type.to_string(),
                    count,
                });
            }
        }

        None
    }
}
```

### Feedback Integration

```rust
pub struct FeedbackIntegrator {
    /// Verification engine
    verifier: VerificationEngine,
    /// Memory store for learnings
    memory: Arc<RwLock<MemoryStore>>,
}

impl FeedbackIntegrator {
    /// Process verification results and update context
    pub async fn integrate(&self, results: &[VerificationResult]) -> Result<FeedbackSummary> {
        let mut memory = self.memory.write().await;
        let mut summary = FeedbackSummary::default();

        for result in results {
            if result.passed {
                summary.passed += 1;

                // Record successful pattern
                if let Some(learning) = self.extract_success_learning(result) {
                    memory.add(learning).await?;
                    summary.learnings_added += 1;
                }
            } else {
                summary.failed += 1;

                // Record error pattern
                let error_memory = MemoryItem {
                    id: MemoryId::new(),
                    memory_type: MemoryType::ShortTerm,
                    content: MemoryContent::ErrorResolution {
                        error_pattern: result.claim.assertion.clone(),
                        root_cause: self.analyze_failure(&result.output),
                        solution: String::new(),  // To be filled when resolved
                    },
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                    access_count: 1,
                    relevance_score: 0.8,
                    tags: vec!["error".to_string(), "unresolved".to_string()],
                    source: MemorySource::Verification,
                };
                memory.add(error_memory).await?;
            }
        }

        Ok(summary)
    }

    /// Update error memory when resolution is found
    pub async fn record_resolution(&self, error_id: &MemoryId, solution: &str) -> Result<()> {
        let mut memory = self.memory.write().await;

        if let Some(item) = memory.items.get_mut(error_id) {
            if let MemoryContent::ErrorResolution { ref mut solution as sol, .. } = &mut item.content {
                *sol = solution.to_string();
            }
            // Promote to long-term memory - this is valuable knowledge
            item.memory_type = MemoryType::LongTerm;
            item.relevance_score = 1.0;
            item.tags.retain(|t| t != "unresolved");
            item.tags.push("resolved".to_string());
        }

        Ok(())
    }
}
```

---

## 6. Drift Detection and Correction

### Drift Signals

```rust
#[derive(Debug, Clone)]
pub enum DriftSignal {
    /// High verification failure rate
    HighFailureRate { rate: f64 },
    /// Repeated failures on same type
    RepeatedFailures { claim_type: String, count: u32 },
    /// Circular reasoning detected
    CircularReasoning { pattern: String },
    /// Contradictory claims
    Contradiction { claim_a: String, claim_b: String },
    /// Tunnel vision (narrow focus)
    TunnelVision { topic_distribution: HashMap<String, f64> },
    /// Risk aversion (only small changes)
    RiskAversion { avg_change_size: f64, threshold: f64 },
    /// Context staleness
    StaleContext { age: Duration },
}

pub struct DriftDetector {
    /// Verification engine
    verifier: Arc<VerificationEngine>,
    /// Memory store
    memory: Arc<RwLock<MemoryStore>>,
    /// Recent actions for pattern detection
    recent_actions: VecDeque<AgentAction>,
    /// Configuration
    config: DriftDetectorConfig,
}

#[derive(Debug, Clone)]
pub struct DriftDetectorConfig {
    /// Failure rate threshold
    pub max_failure_rate: f64,
    /// Actions to track for patterns
    pub action_history_size: usize,
    /// Tunnel vision threshold (max % on single topic)
    pub tunnel_vision_threshold: f64,
    /// Risk aversion change size threshold
    pub risk_aversion_threshold: f64,
    /// Context staleness threshold
    pub staleness_threshold: Duration,
}

impl DriftDetector {
    /// Run comprehensive drift detection
    pub async fn detect(&self) -> Vec<DriftSignal> {
        let mut signals = Vec::new();

        // Check verification patterns
        if let Some(signal) = self.verifier.detect_drift(Duration::from_secs(3600)) {
            signals.push(signal);
        }

        // Check for tunnel vision
        if let Some(signal) = self.detect_tunnel_vision().await {
            signals.push(signal);
        }

        // Check for risk aversion
        if let Some(signal) = self.detect_risk_aversion().await {
            signals.push(signal);
        }

        // Check for circular reasoning
        if let Some(signal) = self.detect_circular_reasoning().await {
            signals.push(signal);
        }

        // Check context staleness
        if let Some(signal) = self.detect_staleness().await {
            signals.push(signal);
        }

        signals
    }

    /// Detect tunnel vision (narrow focus)
    async fn detect_tunnel_vision(&self) -> Option<DriftSignal> {
        let mut topic_counts: HashMap<String, u32> = HashMap::new();

        for action in &self.recent_actions {
            for topic in action.topics() {
                *topic_counts.entry(topic).or_default() += 1;
            }
        }

        let total: u32 = topic_counts.values().sum();
        if total == 0 {
            return None;
        }

        let distribution: HashMap<String, f64> = topic_counts.iter()
            .map(|(k, v)| (k.clone(), *v as f64 / total as f64))
            .collect();

        // Check if any single topic dominates
        for (_, pct) in &distribution {
            if *pct > self.config.tunnel_vision_threshold {
                return Some(DriftSignal::TunnelVision { topic_distribution: distribution });
            }
        }

        None
    }

    /// Detect risk aversion (only small, safe changes)
    async fn detect_risk_aversion(&self) -> Option<DriftSignal> {
        let change_sizes: Vec<f64> = self.recent_actions.iter()
            .filter_map(|a| a.change_size())
            .collect();

        if change_sizes.is_empty() {
            return None;
        }

        let avg_size = change_sizes.iter().sum::<f64>() / change_sizes.len() as f64;

        if avg_size < self.config.risk_aversion_threshold {
            return Some(DriftSignal::RiskAversion {
                avg_change_size: avg_size,
                threshold: self.config.risk_aversion_threshold,
            });
        }

        None
    }

    /// Detect circular reasoning
    async fn detect_circular_reasoning(&self) -> Option<DriftSignal> {
        // Look for repeated similar actions
        let mut action_patterns: HashMap<String, Vec<usize>> = HashMap::new();

        for (i, action) in self.recent_actions.iter().enumerate() {
            let pattern = action.pattern_signature();
            action_patterns.entry(pattern).or_default().push(i);
        }

        // Check for suspicious patterns (same action repeated close together)
        for (pattern, indices) in action_patterns {
            if indices.len() > 3 {
                // Check if indices are close together
                let avg_gap = indices.windows(2)
                    .map(|w| w[1] - w[0])
                    .sum::<usize>() as f64 / (indices.len() - 1) as f64;

                if avg_gap < 5.0 {
                    return Some(DriftSignal::CircularReasoning { pattern });
                }
            }
        }

        None
    }
}
```

### Drift Correction

```rust
pub struct DriftCorrector {
    /// Memory store
    memory: Arc<RwLock<MemoryStore>>,
    /// LLM for reflection
    llm: Arc<dyn LlmClient>,
}

impl DriftCorrector {
    /// Apply corrections based on detected drift signals
    pub async fn correct(&self, signals: &[DriftSignal]) -> Result<CorrectionReport> {
        let mut report = CorrectionReport::default();

        for signal in signals {
            match signal {
                DriftSignal::HighFailureRate { rate } => {
                    // Refresh verification targets
                    self.refresh_verification_context().await?;
                    report.actions.push(CorrectionAction::RefreshedContext);
                }

                DriftSignal::TunnelVision { topic_distribution } => {
                    // Force exploration of neglected areas
                    let neglected = self.identify_neglected_areas(topic_distribution).await?;
                    report.actions.push(CorrectionAction::ForcedExploration { areas: neglected });
                }

                DriftSignal::RiskAversion { .. } => {
                    // Inject prompt to encourage ambitious changes
                    report.actions.push(CorrectionAction::PromptInjection {
                        prompt: "Consider larger, more impactful changes. Small incremental \
                                 changes may indicate tunnel vision.".to_string()
                    });
                }

                DriftSignal::CircularReasoning { pattern } => {
                    // Break the cycle with fresh perspective
                    let fresh_approach = self.generate_fresh_approach(pattern).await?;
                    report.actions.push(CorrectionAction::FreshApproach { approach: fresh_approach });
                }

                DriftSignal::StaleContext { age } => {
                    // Refresh context from source
                    self.refresh_context_from_source().await?;
                    report.actions.push(CorrectionAction::RefreshedContext);
                }

                DriftSignal::Contradiction { claim_a, claim_b } => {
                    // Resolve contradiction
                    let resolution = self.resolve_contradiction(claim_a, claim_b).await?;
                    report.actions.push(CorrectionAction::ResolvedContradiction { resolution });
                }

                _ => {}
            }
        }

        Ok(report)
    }

    /// Generate a fresh approach using reflection
    async fn generate_fresh_approach(&self, stuck_pattern: &str) -> Result<String> {
        let prompt = format!(
            "The agent appears to be stuck in a circular pattern:\n{}\n\n\
             Suggest a completely different approach to break this cycle. \
             Think from first principles about what the actual goal is \
             and how else it might be achieved.",
            stuck_pattern
        );

        let response = self.llm.complete(LlmRequest::new()
            .with_system("You are a meta-reasoning assistant helping an AI agent break out of \
                         circular reasoning patterns.")
            .with_message(Role::User, &prompt)
        ).await?;

        Ok(response.content)
    }
}
```

---

## 7. Context Window Management

### Dynamic Context Assembly

```rust
pub struct ContextAssembler {
    /// Memory store
    memory: Arc<RwLock<MemoryStore>>,
    /// Resource budget
    budget: ResourceBudget,
    /// Tokenizer for estimation
    tokenizer: Arc<dyn Tokenizer>,
}

impl ContextAssembler {
    /// Assemble optimal context for a task
    pub async fn assemble(&self, task: &Task, usage: &ResourceUsage) -> Result<AssembledContext> {
        let remaining_tokens = usage.remaining(&self.budget).context_tokens;

        // Reserve tokens for response
        let available_tokens = remaining_tokens.saturating_sub(4096);

        // Priority-ordered context sections
        let mut sections = Vec::new();
        let mut used_tokens = 0;

        // 1. Critical: Task specification (always include)
        let task_section = self.format_task_section(task);
        let task_tokens = self.tokenizer.count(&task_section);
        sections.push(ContextSection::Task(task_section));
        used_tokens += task_tokens;

        // 2. High: Relevant memories
        let relevant_memories = self.memory.read().await.retrieve(&MemoryQuery {
            tags: task.tags(),
            max_results: Some(20),
            min_relevance: Some(0.5),
        }).await;

        for memory in relevant_memories {
            let memory_text = self.format_memory(memory);
            let memory_tokens = self.tokenizer.count(&memory_text);

            if used_tokens + memory_tokens > available_tokens {
                break;
            }

            sections.push(ContextSection::Memory(memory_text));
            used_tokens += memory_tokens;
        }

        // 3. Medium: File contents
        let file_budget = available_tokens.saturating_sub(used_tokens) / 2;
        let file_sections = self.assemble_file_context(task, file_budget).await?;
        for section in file_sections {
            let section_tokens = self.tokenizer.count(&section.content());
            if used_tokens + section_tokens <= available_tokens {
                sections.push(section);
                used_tokens += section_tokens;
            }
        }

        // 4. Low: Recent history (if space)
        if used_tokens < available_tokens * 8 / 10 {
            let history_budget = available_tokens.saturating_sub(used_tokens);
            let history = self.assemble_history(history_budget).await?;
            sections.push(ContextSection::History(history));
        }

        Ok(AssembledContext {
            sections,
            total_tokens: used_tokens,
            budget_remaining: available_tokens.saturating_sub(used_tokens),
        })
    }

    /// Compress context when near limits
    pub async fn compress(&self, context: &AssembledContext) -> Result<AssembledContext> {
        let mut compressed_sections = Vec::new();

        for section in &context.sections {
            let compressed = match section {
                ContextSection::Memory(text) => {
                    // Summarize memories
                    let summary = self.summarize(text, text.len() / 2).await?;
                    ContextSection::Memory(summary)
                }
                ContextSection::FileContent { path, content } => {
                    // Keep only relevant portions
                    let relevant = self.extract_relevant(content).await?;
                    ContextSection::FileContent {
                        path: path.clone(),
                        content: relevant,
                    }
                }
                ContextSection::History(text) => {
                    // Aggressive summarization
                    let summary = self.summarize(text, text.len() / 3).await?;
                    ContextSection::History(summary)
                }
                other => other.clone(),
            };
            compressed_sections.push(compressed);
        }

        Ok(AssembledContext {
            sections: compressed_sections,
            total_tokens: self.tokenizer.count_sections(&compressed_sections),
            budget_remaining: context.budget_remaining,
        })
    }
}
```

### Incremental Context Updates

```rust
pub struct IncrementalContextManager {
    /// Current context state
    current: AssembledContext,
    /// Pending updates
    pending_updates: Vec<ContextUpdate>,
    /// Assembler
    assembler: Arc<ContextAssembler>,
}

#[derive(Debug, Clone)]
pub enum ContextUpdate {
    /// Add new information
    Add { section: ContextSection, priority: i32 },
    /// Remove outdated information
    Remove { section_id: String },
    /// Update existing section
    Update { section_id: String, new_content: String },
    /// Invalidate section (mark as stale)
    Invalidate { section_id: String, reason: String },
}

impl IncrementalContextManager {
    /// Apply updates without full reassembly
    pub async fn apply_updates(&mut self) -> Result<()> {
        let mut updates_applied = 0;

        for update in self.pending_updates.drain(..) {
            match update {
                ContextUpdate::Add { section, priority } => {
                    // Check if we have room
                    let section_tokens = self.assembler.tokenizer.count(&section.content());
                    if self.current.budget_remaining >= section_tokens {
                        self.current.sections.push(section);
                        self.current.total_tokens += section_tokens;
                        self.current.budget_remaining -= section_tokens;
                        updates_applied += 1;
                    } else {
                        // Need to compress or remove lower priority items
                        self.make_room(section_tokens).await?;
                        // Retry add
                        if self.current.budget_remaining >= section_tokens {
                            self.current.sections.push(section);
                            self.current.total_tokens += section_tokens;
                            self.current.budget_remaining -= section_tokens;
                            updates_applied += 1;
                        }
                    }
                }
                ContextUpdate::Remove { section_id } => {
                    if let Some(idx) = self.current.sections.iter()
                        .position(|s| s.id() == section_id)
                    {
                        let removed = self.current.sections.remove(idx);
                        let tokens = self.assembler.tokenizer.count(&removed.content());
                        self.current.total_tokens -= tokens;
                        self.current.budget_remaining += tokens;
                        updates_applied += 1;
                    }
                }
                ContextUpdate::Update { section_id, new_content } => {
                    if let Some(section) = self.current.sections.iter_mut()
                        .find(|s| s.id() == section_id)
                    {
                        let old_tokens = self.assembler.tokenizer.count(&section.content());
                        let new_tokens = self.assembler.tokenizer.count(&new_content);
                        section.set_content(new_content);
                        self.current.total_tokens = self.current.total_tokens - old_tokens + new_tokens;
                        self.current.budget_remaining = self.current.budget_remaining + old_tokens - new_tokens;
                        updates_applied += 1;
                    }
                }
                ContextUpdate::Invalidate { section_id, reason } => {
                    // Mark as stale but keep for now
                    if let Some(section) = self.current.sections.iter_mut()
                        .find(|s| s.id() == section_id)
                    {
                        section.mark_stale(&reason);
                        updates_applied += 1;
                    }
                }
            }
        }

        tracing::info!(updates_applied, "Applied context updates");
        Ok(())
    }
}
```

---

## 8. Strategy Adaptation

### Dynamic Strategy Selection

Based on CodeAdapt's finding that models perform better when they can adapt strategy based on task type and intermediate results.

```rust
#[derive(Debug, Clone)]
pub enum ReasoningStrategy {
    /// Single-turn reasoning for simple tasks
    ChainOfThought,
    /// Code-centric with execution
    ProgramOfThought,
    /// Iterative decomposition
    StepwiseRefinement,
    /// Try multiple approaches
    ParallelExploration { branches: u32 },
    /// Verify each step
    VerifiedReasoning,
}

pub struct StrategySelector {
    /// Task classifier
    classifier: TaskClassifier,
    /// Performance history
    history: HashMap<(TaskType, ReasoningStrategy), PerformanceStats>,
}

impl StrategySelector {
    /// Select strategy based on task and history
    pub fn select(&self, task: &Task, resources: &ResourceUsage) -> ReasoningStrategy {
        let task_type = self.classifier.classify(task);

        // Check if resources are constrained
        let resource_pressure = resources.is_near_limit(
            &ResourceBudget::default(),
            0.7
        );

        if resource_pressure.any() {
            // Use simpler strategy when resources are tight
            return ReasoningStrategy::ChainOfThought;
        }

        // Use historical performance
        if let Some(best) = self.best_strategy_for_type(&task_type) {
            return best;
        }

        // Default strategies by task type
        match task_type {
            TaskType::BugFix { .. } => ReasoningStrategy::VerifiedReasoning,
            TaskType::Feature { .. } => ReasoningStrategy::StepwiseRefinement,
            TaskType::Refactor { .. } => ReasoningStrategy::ProgramOfThought,
            TaskType::Test { .. } => ReasoningStrategy::ProgramOfThought,
            TaskType::Exploration { .. } => ReasoningStrategy::ParallelExploration { branches: 3 },
            _ => ReasoningStrategy::ChainOfThought,
        }
    }

    /// Adapt strategy based on intermediate results
    pub fn adapt(&self, current: &ReasoningStrategy, feedback: &StrategyFeedback) -> Option<ReasoningStrategy> {
        match (current, feedback) {
            // If verification is failing, switch to more careful approach
            (_, StrategyFeedback::HighFailureRate) => {
                Some(ReasoningStrategy::VerifiedReasoning)
            }

            // If stuck, try parallel exploration
            (ReasoningStrategy::ChainOfThought, StrategyFeedback::NoProgress) |
            (ReasoningStrategy::StepwiseRefinement, StrategyFeedback::NoProgress) => {
                Some(ReasoningStrategy::ParallelExploration { branches: 2 })
            }

            // If taking too long, simplify
            (ReasoningStrategy::ParallelExploration { .. }, StrategyFeedback::ResourcePressure) => {
                Some(ReasoningStrategy::ChainOfThought)
            }

            _ => None,
        }
    }
}
```

---

## 9. Database Schema

```sql
-- Memory items
CREATE TABLE memory_items (
    id TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL,  -- working, short_term, long_term, episodic
    content_type TEXT NOT NULL,  -- code_pattern, decision, error_resolution, etc.
    content TEXT NOT NULL,  -- JSON
    created_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    relevance_score REAL NOT NULL DEFAULT 1.0,
    tags TEXT NOT NULL,  -- JSON array
    source_type TEXT NOT NULL,
    source_id TEXT
);

-- Verification history
CREATE TABLE verifications (
    id TEXT PRIMARY KEY,
    claim_type TEXT NOT NULL,
    assertion TEXT NOT NULL,
    passed INTEGER NOT NULL,
    output TEXT NOT NULL,
    verified_at TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    task_id TEXT REFERENCES tasks(id)
);

-- Drift signals
CREATE TABLE drift_signals (
    id TEXT PRIMARY KEY,
    signal_type TEXT NOT NULL,
    details TEXT NOT NULL,  -- JSON
    detected_at TEXT NOT NULL,
    corrected_at TEXT,
    correction_action TEXT  -- JSON
);

-- Strategy performance
CREATE TABLE strategy_performance (
    task_type TEXT NOT NULL,
    strategy TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    successes INTEGER NOT NULL DEFAULT 0,
    avg_duration_ms INTEGER,
    avg_tokens_used INTEGER,
    PRIMARY KEY (task_type, strategy)
);

-- Context snapshots (for recovery)
CREATE TABLE context_snapshots (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    assembled_at TEXT NOT NULL,
    total_tokens INTEGER NOT NULL,
    sections TEXT NOT NULL,  -- JSON
    resource_usage TEXT NOT NULL  -- JSON
);

-- Indexes
CREATE INDEX idx_memory_items_type ON memory_items(memory_type);
CREATE INDEX idx_memory_items_relevance ON memory_items(relevance_score DESC);
CREATE INDEX idx_verifications_task ON verifications(task_id);
CREATE INDEX idx_drift_signals_type ON drift_signals(signal_type);
```

---

## 10. Integration with Multi-Agent System

### Context Manager per Agent

```rust
impl DomainPlanner {
    /// Context manager for this planner
    context_manager: ContextManager,

    /// Handle task completion with context refresh
    async fn on_task_completed(&mut self, completion: TaskCompletion) -> Result<Vec<Task>> {
        // Absorb learnings into memory
        self.context_manager.absorb_learnings(&completion).await?;

        // Check for drift
        let drift_signals = self.context_manager.detect_drift().await;
        if !drift_signals.is_empty() {
            // Apply corrections
            self.context_manager.correct_drift(&drift_signals).await?;
        }

        // Refresh context before planning next tasks
        self.context_manager.refresh().await?;

        // Continue with planning...
        self.plan_next_tasks(&completion).await
    }
}

impl Worker {
    /// Context manager for this worker
    context_manager: ContextManager,

    /// Execute with resource awareness
    async fn execute(&mut self, task: Task) -> Result<TaskCompletion> {
        let budget = self.context_manager.budget();
        let mut usage = ResourceUsage::default();

        loop {
            // Check resource limits
            if usage.is_near_limit(&budget, 0.9).any() {
                // Need to wrap up
                return self.wrap_up_task(&task, &usage).await;
            }

            // Assemble context for this step
            let context = self.context_manager.assemble(&task, &usage).await?;

            // Execute step
            let step_result = self.execute_step(&task, &context).await?;

            // Verify step
            let verifications = self.context_manager.verify(&step_result.claims).await;

            // Update usage
            usage.reasoning_steps += 1;
            usage.context_tokens = context.total_tokens;

            // Check if done
            if step_result.is_complete() {
                return self.finalize_task(&task, &step_result).await;
            }

            // Integrate feedback
            self.context_manager.integrate_feedback(&verifications).await?;
        }
    }
}
```

---

## 11. Implementation Checklist

### Phase 1: Core Memory System

- [ ] Create `loom-context` crate
- [ ] Implement `MemoryStore` with types
- [ ] Implement decay and consolidation
- [ ] Add database migrations
- [ ] Implement memory retrieval queries

### Phase 2: Resource Management

- [ ] Implement `ResourceBudget` and `ResourceUsage`
- [ ] Add resource-aware prompting
- [ ] Implement context assembly with budget
- [ ] Add compression for budget overflow

### Phase 3: Verification

- [ ] Implement `VerificationEngine`
- [ ] Add verifiers for each claim type
- [ ] Implement feedback integration
- [ ] Add verification history tracking

### Phase 4: Drift Detection

- [ ] Implement `DriftDetector` with all signal types
- [ ] Implement `DriftCorrector`
- [ ] Add correction strategies
- [ ] Integrate with planner/worker loops

### Phase 5: Strategy Selection

- [ ] Implement `StrategySelector`
- [ ] Add performance tracking
- [ ] Implement dynamic adaptation
- [ ] Add strategy-specific prompt builders

---

## 12. Future Enhancements

- Cross-agent memory sharing with relevance filtering
- Learned compression models for better summarization
- Predictive resource allocation based on task complexity
- External knowledge base integration
- Memory consolidation during idle periods
- Attention-based relevance scoring
