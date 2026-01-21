// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Back-pressure controller for handling planner overload.
//!
//! This module provides a controller that manages event delivery to planners,
//! ensuring graceful degradation when planners are overwhelmed with events.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use loom_multi_agent_core::PlannerId;
use tracing::warn;

use crate::events::{EventPriority, TaskEvent};

/// Decision made by the back-pressure controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureDecision {
	/// Accept the event for delivery
	Accept,
	/// Accept with a warning (queue depth is high)
	AcceptWithWarning,
	/// Defer the event for later delivery
	Defer,
	/// Reject the event (queue at maximum capacity)
	Reject,
}

/// Thresholds for back-pressure handling.
#[derive(Debug, Clone)]
pub struct BackpressureThresholds {
	/// Warning threshold - start logging warnings
	pub warning: usize,
	/// Critical threshold - only accept high-priority events
	pub critical: usize,
	/// Maximum threshold - reject all new events
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

/// Metrics for the back-pressure controller.
#[derive(Debug, Default)]
pub struct BackpressureMetrics {
	/// Total events accepted
	pub events_accepted: AtomicU64,
	/// Events accepted with warning
	pub events_warned: AtomicU64,
	/// Events deferred
	pub events_deferred: AtomicU64,
	/// Events rejected
	pub events_rejected: AtomicU64,
}

impl BackpressureMetrics {
	pub fn accepted(&self) -> u64 {
		self.events_accepted.load(Ordering::Relaxed)
	}

	pub fn warned(&self) -> u64 {
		self.events_warned.load(Ordering::Relaxed)
	}

	pub fn deferred(&self) -> u64 {
		self.events_deferred.load(Ordering::Relaxed)
	}

	pub fn rejected(&self) -> u64 {
		self.events_rejected.load(Ordering::Relaxed)
	}
}

/// Controller for managing back-pressure in event delivery.
pub struct BackpressureController {
	/// Queue depth thresholds
	thresholds: BackpressureThresholds,
	/// Current queue depths per planner
	queue_depths: HashMap<PlannerId, usize>,
	/// Metrics
	metrics: BackpressureMetrics,
}

impl BackpressureController {
	/// Create a new back-pressure controller with default thresholds.
	pub fn new() -> Self {
		Self::with_thresholds(BackpressureThresholds::default())
	}

	/// Create a new back-pressure controller with custom thresholds.
	pub fn with_thresholds(thresholds: BackpressureThresholds) -> Self {
		Self {
			thresholds,
			queue_depths: HashMap::new(),
			metrics: BackpressureMetrics::default(),
		}
	}

	/// Get the metrics for this controller.
	pub fn metrics(&self) -> &BackpressureMetrics {
		&self.metrics
	}

	/// Get the current queue depth for a planner.
	pub fn queue_depth(&self, planner_id: &PlannerId) -> usize {
		self.queue_depths.get(planner_id).copied().unwrap_or(0)
	}

	/// Check if a planner can accept more events.
	pub fn can_accept(&self, planner_id: &PlannerId, event: &TaskEvent) -> BackpressureDecision {
		let depth = self.queue_depth(planner_id);

		if depth >= self.thresholds.maximum {
			return BackpressureDecision::Reject;
		}

		if depth >= self.thresholds.critical {
			if event.priority() >= EventPriority::High {
				return BackpressureDecision::AcceptWithWarning;
			}
			return BackpressureDecision::Defer;
		}

		if depth >= self.thresholds.warning {
			return BackpressureDecision::AcceptWithWarning;
		}

		BackpressureDecision::Accept
	}

	/// Record that an event was accepted and increment queue depth.
	pub fn record_accepted(&mut self, planner_id: &PlannerId, decision: BackpressureDecision) {
		*self.queue_depths.entry(planner_id.clone()).or_default() += 1;

		match decision {
			BackpressureDecision::Accept => {
				self.metrics.events_accepted.fetch_add(1, Ordering::Relaxed);
			}
			BackpressureDecision::AcceptWithWarning => {
				self.metrics.events_warned.fetch_add(1, Ordering::Relaxed);
				warn!(
					planner_id = %planner_id,
					depth = self.queue_depth(planner_id),
					"Queue depth warning threshold exceeded"
				);
			}
			BackpressureDecision::Defer => {
				self.metrics.events_deferred.fetch_add(1, Ordering::Relaxed);
			}
			BackpressureDecision::Reject => {
				self.metrics.events_rejected.fetch_add(1, Ordering::Relaxed);
			}
		}
	}

	/// Record that an event was deferred.
	pub fn record_deferred(&mut self) {
		self.metrics.events_deferred.fetch_add(1, Ordering::Relaxed);
	}

	/// Record that an event was rejected.
	pub fn record_rejected(&mut self, planner_id: &PlannerId) {
		self.metrics.events_rejected.fetch_add(1, Ordering::Relaxed);
		warn!(
			planner_id = %planner_id,
			depth = self.queue_depth(planner_id),
			"Event rejected due to back-pressure"
		);
	}

	/// Decrement queue depth when an event is processed.
	pub fn event_processed(&mut self, planner_id: &PlannerId) {
		if let Some(depth) = self.queue_depths.get_mut(planner_id) {
			*depth = depth.saturating_sub(1);
		}
	}

	/// Reset the queue depth for a planner.
	pub fn reset_planner(&mut self, planner_id: &PlannerId) {
		self.queue_depths.remove(planner_id);
	}

	/// Check if a planner is overloaded.
	pub fn is_overloaded(&self, planner_id: &PlannerId) -> bool {
		self.queue_depth(planner_id) >= self.thresholds.critical
	}

	/// Get total active planners.
	pub fn active_planners(&self) -> usize {
		self.queue_depths.len()
	}
}

impl Default for BackpressureController {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_backpressure_accept() {
		let controller = BackpressureController::new();
		let planner_id = PlannerId::new();
		let event = TaskEvent::Created {
			task_id: loom_multi_agent_core::TaskId::new(),
			planner_id: planner_id.clone(),
			description: "Test".to_string(),
		};

		let decision = controller.can_accept(&planner_id, &event);
		assert_eq!(decision, BackpressureDecision::Accept);
	}

	#[test]
	fn test_backpressure_warning() {
		let thresholds = BackpressureThresholds {
			warning: 5,
			critical: 10,
			maximum: 20,
		};
		let mut controller = BackpressureController::with_thresholds(thresholds);
		let planner_id = PlannerId::new();

		for _ in 0..6 {
			controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		}

		let event = TaskEvent::Created {
			task_id: loom_multi_agent_core::TaskId::new(),
			planner_id: planner_id.clone(),
			description: "Test".to_string(),
		};

		let decision = controller.can_accept(&planner_id, &event);
		assert_eq!(decision, BackpressureDecision::AcceptWithWarning);
	}

	#[test]
	fn test_backpressure_defer_low_priority() {
		let thresholds = BackpressureThresholds {
			warning: 5,
			critical: 10,
			maximum: 20,
		};
		let mut controller = BackpressureController::with_thresholds(thresholds);
		let planner_id = PlannerId::new();

		for _ in 0..11 {
			controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		}

		let event = TaskEvent::Progress {
			task_id: loom_multi_agent_core::TaskId::new(),
			worker_id: loom_multi_agent_core::WorkerId::new(),
			percentage: 50,
			current_step: "Test".to_string(),
		};

		let decision = controller.can_accept(&planner_id, &event);
		assert_eq!(decision, BackpressureDecision::Defer);
	}

	#[test]
	fn test_backpressure_accept_high_priority_at_critical() {
		let thresholds = BackpressureThresholds {
			warning: 5,
			critical: 10,
			maximum: 20,
		};
		let mut controller = BackpressureController::with_thresholds(thresholds);
		let planner_id = PlannerId::new();

		for _ in 0..11 {
			controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		}

		let event = TaskEvent::Failed {
			task_id: loom_multi_agent_core::TaskId::new(),
			worker_id: loom_multi_agent_core::WorkerId::new(),
			error: "Test error".to_string(),
		};

		let decision = controller.can_accept(&planner_id, &event);
		assert_eq!(decision, BackpressureDecision::AcceptWithWarning);
	}

	#[test]
	fn test_backpressure_reject() {
		let thresholds = BackpressureThresholds {
			warning: 5,
			critical: 10,
			maximum: 20,
		};
		let mut controller = BackpressureController::with_thresholds(thresholds);
		let planner_id = PlannerId::new();

		for _ in 0..21 {
			controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		}

		let event = TaskEvent::Created {
			task_id: loom_multi_agent_core::TaskId::new(),
			planner_id: planner_id.clone(),
			description: "Test".to_string(),
		};

		let decision = controller.can_accept(&planner_id, &event);
		assert_eq!(decision, BackpressureDecision::Reject);
	}

	#[test]
	fn test_event_processed() {
		let mut controller = BackpressureController::new();
		let planner_id = PlannerId::new();

		controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		assert_eq!(controller.queue_depth(&planner_id), 2);

		controller.event_processed(&planner_id);
		assert_eq!(controller.queue_depth(&planner_id), 1);
	}

	#[test]
	fn test_is_overloaded() {
		let thresholds = BackpressureThresholds {
			warning: 5,
			critical: 10,
			maximum: 20,
		};
		let mut controller = BackpressureController::with_thresholds(thresholds);
		let planner_id = PlannerId::new();

		assert!(!controller.is_overloaded(&planner_id));

		for _ in 0..10 {
			controller.record_accepted(&planner_id, BackpressureDecision::Accept);
		}

		assert!(controller.is_overloaded(&planner_id));
	}
}
