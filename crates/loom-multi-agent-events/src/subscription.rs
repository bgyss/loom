// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Event subscription with filtering.

use std::time::Duration;

use loom_multi_agent_core::{PlannerId, TaskId, WorkerId};
use tokio::sync::broadcast;
use tokio::time::timeout;

use crate::events::TaskEvent;
use crate::{EventError, Result};

/// Filter for event subscriptions.
#[derive(Debug, Clone)]
pub enum EventFilter {
	/// Receive all events
	All,
	/// Only events for a specific task
	Task(TaskId),
	/// Only events from a specific planner
	Planner(PlannerId),
	/// Only events from a specific worker
	Worker(WorkerId),
	/// Only completion events
	Completions,
	/// Only failure events
	Failures,
	/// Custom filter function (by event type)
	Custom(Vec<EventType>),
}

/// Event types for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
	Created,
	Assigned,
	Started,
	Progress,
	Completed,
	Failed,
	Cancelled,
	NeedsIteration,
}

impl EventFilter {
	/// Check if an event matches this filter.
	#[must_use]
	pub fn matches(&self, event: &TaskEvent) -> bool {
		match self {
			EventFilter::All => true,
			EventFilter::Task(task_id) => event.task_id() == task_id,
			EventFilter::Planner(planner_id) => {
				if let TaskEvent::Created {
					planner_id: pid, ..
				} = event
				{
					pid == planner_id
				} else {
					false
				}
			}
			EventFilter::Worker(worker_id) => match event {
				TaskEvent::Assigned { worker_id: wid, .. } => wid == worker_id,
				TaskEvent::Started { worker_id: wid, .. } => wid == worker_id,
				TaskEvent::Progress { worker_id: wid, .. } => wid == worker_id,
				TaskEvent::Completed(c) => &c.worker_id == worker_id,
				TaskEvent::Failed { worker_id: wid, .. } => wid == worker_id,
				_ => false,
			},
			EventFilter::Completions => matches!(event, TaskEvent::Completed(_)),
			EventFilter::Failures => matches!(event, TaskEvent::Failed { .. }),
			EventFilter::Custom(types) => {
				let event_type = match event {
					TaskEvent::Created { .. } => EventType::Created,
					TaskEvent::Assigned { .. } => EventType::Assigned,
					TaskEvent::Started { .. } => EventType::Started,
					TaskEvent::Progress { .. } => EventType::Progress,
					TaskEvent::Completed(_) => EventType::Completed,
					TaskEvent::Failed { .. } => EventType::Failed,
					TaskEvent::Cancelled { .. } => EventType::Cancelled,
					TaskEvent::NeedsIteration { .. } => EventType::NeedsIteration,
				};
				types.contains(&event_type)
			}
		}
	}
}

/// A subscription to events with optional filtering.
pub struct EventSubscription {
	/// The broadcast receiver
	rx: broadcast::Receiver<TaskEvent>,
	/// Filter to apply
	filter: EventFilter,
}

impl EventSubscription {
	/// Create a new subscription.
	pub(crate) fn new(rx: broadcast::Receiver<TaskEvent>, filter: EventFilter) -> Self {
		Self { rx, filter }
	}

	/// Receive the next event that matches the filter.
	pub async fn recv(&mut self) -> Result<TaskEvent> {
		loop {
			match self.rx.recv().await {
				Ok(event) => {
					if self.filter.matches(&event) {
						return Ok(event);
					}
				}
				Err(broadcast::error::RecvError::Closed) => {
					return Err(EventError::ChannelClosed);
				}
				Err(broadcast::error::RecvError::Lagged(n)) => {
					tracing::warn!("Event subscription lagged by {} events", n);
				}
			}
		}
	}

	/// Receive the next event with a timeout.
	pub async fn recv_timeout(&mut self, duration: Duration) -> Result<TaskEvent> {
		timeout(duration, self.recv())
			.await
			.map_err(|_| EventError::Timeout)?
	}

	/// Try to receive an event without blocking.
	pub fn try_recv(&mut self) -> Option<TaskEvent> {
		loop {
			match self.rx.try_recv() {
				Ok(event) => {
					if self.filter.matches(&event) {
						return Some(event);
					}
				}
				Err(_) => return None,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_event_filter_all() {
		let filter = EventFilter::All;
		let event = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		assert!(filter.matches(&event));
	}

	#[test]
	fn test_event_filter_task() {
		let task_id = TaskId::new();
		let filter = EventFilter::Task(task_id.clone());

		let matching = TaskEvent::Created {
			task_id: task_id.clone(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		assert!(filter.matches(&matching));

		let non_matching = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Other".to_string(),
		};
		assert!(!filter.matches(&non_matching));
	}

	#[test]
	fn test_event_filter_completions() {
		let filter = EventFilter::Completions;

		let created = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		assert!(!filter.matches(&created));
	}

	#[test]
	fn test_event_filter_custom() {
		let filter = EventFilter::Custom(vec![EventType::Created, EventType::Completed]);

		let created = TaskEvent::Created {
			task_id: TaskId::new(),
			planner_id: PlannerId::new(),
			description: "Test".to_string(),
		};
		assert!(filter.matches(&created));

		let assigned = TaskEvent::Assigned {
			task_id: TaskId::new(),
			worker_id: WorkerId::new(),
		};
		assert!(!filter.matches(&assigned));
	}
}
