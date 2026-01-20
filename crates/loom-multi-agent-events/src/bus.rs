// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event bus for broadcasting events to subscribers.

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::instrument;

use crate::events::TaskEvent;
use crate::subscription::{EventFilter, EventSubscription};
use crate::{EventError, Result};

/// Default channel capacity for the event bus.
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Event bus for broadcasting events to all subscribers.
#[derive(Debug, Clone)]
pub struct EventBus {
	/// Broadcast sender for task events
	task_tx: broadcast::Sender<TaskEvent>,
}

impl EventBus {
	/// Create a new event bus with default capacity.
	#[must_use]
	pub fn new() -> Self {
		Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
	}

	/// Create a new event bus with specified capacity.
	#[must_use]
	pub fn with_capacity(capacity: usize) -> Self {
		let (task_tx, _) = broadcast::channel(capacity);
		Self { task_tx }
	}

	/// Publish a task event to all subscribers.
	#[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(&event)))]
	pub fn publish(&self, event: TaskEvent) -> Result<usize> {
		self.task_tx.send(event).map_err(|e| {
			tracing::warn!("Failed to publish event: {}", e);
			EventError::SendFailed(e.to_string())
		})
	}

	/// Subscribe to all task events.
	#[must_use]
	pub fn subscribe(&self) -> EventSubscription {
		EventSubscription::new(self.task_tx.subscribe(), EventFilter::All)
	}

	/// Subscribe to task events matching a filter.
	#[must_use]
	pub fn subscribe_filtered(&self, filter: EventFilter) -> EventSubscription {
		EventSubscription::new(self.task_tx.subscribe(), filter)
	}

	/// Get the number of active subscribers.
	#[must_use]
	pub fn subscriber_count(&self) -> usize {
		self.task_tx.receiver_count()
	}
}

impl Default for EventBus {
	fn default() -> Self {
		Self::new()
	}
}

/// Shared event bus handle.
pub type SharedEventBus = Arc<EventBus>;

#[cfg(test)]
mod tests {
	use super::*;
	use loom_multi_agent_core::{PlannerId, TaskId};

	#[tokio::test]
	async fn test_event_bus_publish_subscribe() {
		let bus = EventBus::new();
		let mut sub = bus.subscribe();

		let event = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test task".to_string(),
		};

		let count = bus.publish(event.clone()).unwrap();
		assert_eq!(count, 1);

		let received = sub.recv().await.unwrap();
		assert!(matches!(received, TaskEvent::Created { .. }));
	}

	#[tokio::test]
	async fn test_event_bus_multiple_subscribers() {
		let bus = EventBus::new();
		let mut sub1 = bus.subscribe();
		let mut sub2 = bus.subscribe();

		let event = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};

		let count = bus.publish(event).unwrap();
		assert_eq!(count, 2);

		let r1 = sub1.recv().await.unwrap();
		let r2 = sub2.recv().await.unwrap();

		assert!(matches!(r1, TaskEvent::Created { .. }));
		assert!(matches!(r2, TaskEvent::Created { .. }));
	}

	#[test]
	fn test_event_bus_subscriber_count() {
		let bus = EventBus::new();
		assert_eq!(bus.subscriber_count(), 0);

		let _sub1 = bus.subscribe();
		assert_eq!(bus.subscriber_count(), 1);

		let _sub2 = bus.subscribe();
		assert_eq!(bus.subscriber_count(), 2);
	}
}
