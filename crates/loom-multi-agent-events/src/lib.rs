// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event system for the Loom multi-agent coordination system.
//!
//! This crate provides the event bus and event types that enable
//! communication between agents in the multi-agent system.
//!
//! ## Key Components
//!
//! - [`EventBus`] - Broadcast channel for events
//! - [`TaskEvent`] - Events related to task lifecycle
//! - [`PlannerEvent`] - Events that wake up planners
//! - [`EventSubscription`] - Filtered event subscription

pub mod backpressure;
pub mod bus;
pub mod events;
#[cfg(feature = "persistence")]
pub mod persistence;
pub mod subscription;

pub use backpressure::{BackpressureController, BackpressureDecision, BackpressureThresholds};
pub use bus::{EventBus, EventBusMetrics, SharedEventBus};
pub use events::{
	DependencyResolvedEvent, EventPriority, OrchestratorEvent, PlannerEvent, TaskEvent,
	TaskEventType, WakeUpContext, WorkerEvent,
};
#[cfg(feature = "persistence")]
pub use persistence::{EventHistoryQuery, EventHistoryResult, EventPersistence, StoredEvent};
pub use subscription::{EventFilter, EventSubscription, EventType};

/// Result type for event operations.
pub type Result<T> = std::result::Result<T, EventError>;

/// Errors that can occur in the event system.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
	/// Channel closed
	#[error("event channel closed")]
	ChannelClosed,

	/// Send failed
	#[error("failed to send event: {0}")]
	SendFailed(String),

	/// Receive timeout
	#[error("receive timeout")]
	Timeout,

	/// Serialization error
	#[error("serialization error: {0}")]
	Serialization(String),
}
