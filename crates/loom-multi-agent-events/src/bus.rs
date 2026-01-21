// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event bus for broadcasting events to subscribers.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[cfg(feature = "persistence")]
use loom_multi_agent_core::ProjectRunId;
use tokio::sync::{broadcast, RwLock};
use tracing::instrument;

use crate::backpressure::{BackpressureController, BackpressureDecision};
use crate::events::TaskEvent;
#[cfg(feature = "persistence")]
use crate::persistence::EventPersistence;
use crate::subscription::{EventFilter, EventSubscription};
use crate::{EventError, Result};

/// Default channel capacity for the event bus.
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Metrics for the event bus.
#[derive(Debug, Default)]
pub struct EventBusMetrics {
	/// Total events published
	pub events_published: AtomicU64,
	/// Total events delivered
	pub events_delivered: AtomicU64,
	/// Events deferred due to back-pressure
	pub events_deferred: AtomicU64,
}

impl EventBusMetrics {
	pub fn published(&self) -> u64 {
		self.events_published.load(Ordering::Relaxed)
	}

	pub fn delivered(&self) -> u64 {
		self.events_delivered.load(Ordering::Relaxed)
	}

	pub fn deferred(&self) -> u64 {
		self.events_deferred.load(Ordering::Relaxed)
	}
}

/// Event bus for broadcasting events to all subscribers.
pub struct EventBus {
	/// Broadcast sender for task events
	task_tx: broadcast::Sender<TaskEvent>,
	/// Optional persistence layer
	#[cfg(feature = "persistence")]
	persistence: Option<Arc<EventPersistence>>,
	/// Back-pressure controller
	backpressure: RwLock<BackpressureController>,
	/// Metrics
	metrics: EventBusMetrics,
}

impl std::fmt::Debug for EventBus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("EventBus")
			.field("subscriber_count", &self.task_tx.receiver_count())
			.field("events_published", &self.metrics.published())
			.finish()
	}
}

impl Clone for EventBus {
	fn clone(&self) -> Self {
		Self {
			task_tx: self.task_tx.clone(),
			#[cfg(feature = "persistence")]
			persistence: None,
			backpressure: RwLock::new(BackpressureController::new()),
			metrics: EventBusMetrics::default(),
		}
	}
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
		Self {
			task_tx,
			#[cfg(feature = "persistence")]
			persistence: None,
			backpressure: RwLock::new(BackpressureController::new()),
			metrics: EventBusMetrics::default(),
		}
	}

	/// Create a new event bus with persistence.
	#[cfg(feature = "persistence")]
	pub fn with_persistence(persistence: Arc<EventPersistence>) -> Self {
		let (task_tx, _) = broadcast::channel(DEFAULT_CHANNEL_CAPACITY);
		Self {
			task_tx,
			persistence: Some(persistence),
			backpressure: RwLock::new(BackpressureController::new()),
			metrics: EventBusMetrics::default(),
		}
	}

	/// Get the event bus metrics.
	pub fn metrics(&self) -> &EventBusMetrics {
		&self.metrics
	}

	/// Publish a task event to all subscribers.
	#[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(&event)))]
	pub fn publish(&self, event: TaskEvent) -> Result<usize> {
		self.metrics.events_published.fetch_add(1, Ordering::Relaxed);

		match self.task_tx.send(event) {
			Ok(receivers) => {
				self.metrics
					.events_delivered
					.fetch_add(receivers as u64, Ordering::Relaxed);
				Ok(receivers)
			}
			Err(e) => {
				tracing::warn!("Failed to publish event: {}", e);
				Err(EventError::SendFailed(e.to_string()))
			}
		}
	}

	/// Publish a task event with persistence and project run context.
	#[cfg(feature = "persistence")]
	#[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(&event)))]
	pub async fn publish_with_run(
		&self,
		event: TaskEvent,
		run_id: &ProjectRunId,
	) -> Result<usize> {
		if let Some(ref persistence) = self.persistence {
			persistence.store(&event, run_id).await?;
		}

		self.publish(event)
	}

	/// Publish with back-pressure awareness for a specific planner.
	#[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(&event)))]
	pub async fn publish_with_backpressure(&self, event: TaskEvent) -> Result<BackpressureDecision> {
		let decision = if let Some(planner_id) = event.planner_id() {
			let controller = self.backpressure.read().await;
			controller.can_accept(planner_id, &event)
		} else {
			BackpressureDecision::Accept
		};

		match decision {
			BackpressureDecision::Accept | BackpressureDecision::AcceptWithWarning => {
				if let Some(planner_id) = event.planner_id() {
					let mut controller = self.backpressure.write().await;
					controller.record_accepted(planner_id, decision);
				}

				self.publish(event)?;
			}
			BackpressureDecision::Defer => {
				self.metrics.events_deferred.fetch_add(1, Ordering::Relaxed);
				let mut controller = self.backpressure.write().await;
				controller.record_deferred();
			}
			BackpressureDecision::Reject => {
				if let Some(planner_id) = event.planner_id() {
					let mut controller = self.backpressure.write().await;
					controller.record_rejected(planner_id);
				}
			}
		}

		Ok(decision)
	}

	/// Publish with back-pressure awareness and persistence.
	#[cfg(feature = "persistence")]
	#[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(&event)))]
	pub async fn publish_with_persistence(
		&self,
		event: TaskEvent,
		run_id: &ProjectRunId,
	) -> Result<BackpressureDecision> {
		let decision = if let Some(planner_id) = event.planner_id() {
			let controller = self.backpressure.read().await;
			controller.can_accept(planner_id, &event)
		} else {
			BackpressureDecision::Accept
		};

		match decision {
			BackpressureDecision::Accept | BackpressureDecision::AcceptWithWarning => {
				if let Some(ref persistence) = self.persistence {
					persistence.store(&event, run_id).await?;
				}

				if let Some(planner_id) = event.planner_id() {
					let mut controller = self.backpressure.write().await;
					controller.record_accepted(planner_id, decision);
				}

				self.publish(event)?;
			}
			BackpressureDecision::Defer => {
				self.metrics.events_deferred.fetch_add(1, Ordering::Relaxed);
				let mut controller = self.backpressure.write().await;
				controller.record_deferred();
			}
			BackpressureDecision::Reject => {
				if let Some(planner_id) = event.planner_id() {
					let mut controller = self.backpressure.write().await;
					controller.record_rejected(planner_id);
				}
			}
		}

		Ok(decision)
	}

	/// Mark an event as processed (decrements back-pressure queue depth).
	pub async fn event_processed(&self, planner_id: &loom_multi_agent_core::PlannerId) {
		let mut controller = self.backpressure.write().await;
		controller.event_processed(planner_id);
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

	/// Get the persistence layer if available.
	#[cfg(feature = "persistence")]
	pub fn persistence(&self) -> Option<&Arc<EventPersistence>> {
		self.persistence.as_ref()
	}

	/// Flush any buffered events to persistent storage.
	#[cfg(feature = "persistence")]
	pub async fn flush(&self) -> Result<()> {
		if let Some(ref persistence) = self.persistence {
			persistence.flush().await?;
		}
		Ok(())
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

	#[test]
	fn test_event_bus_metrics() {
		let bus = EventBus::new();
		let _sub = bus.subscribe();

		let event = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};

		let _ = bus.publish(event);
		assert_eq!(bus.metrics().published(), 1);
		assert_eq!(bus.metrics().delivered(), 1);
	}

	#[tokio::test]
	async fn test_event_bus_backpressure() {
		let bus = EventBus::new();
		let _sub = bus.subscribe();

		let event = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};

		let decision = bus.publish_with_backpressure(event).await.unwrap();
		assert_eq!(decision, BackpressureDecision::Accept);
	}
}
