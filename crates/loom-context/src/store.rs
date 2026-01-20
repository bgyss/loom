// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! In-memory memory store with decay and consolidation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use loom_context_core::{
	MemoryId, MemoryItem, MemoryQuery, MemoryType, OrgId, ResourceBudget,
};
use tokio::sync::RwLock;

use crate::error::{ContextError, Result};

/// Configuration for memory decay.
#[derive(Debug, Clone)]
pub struct DecayConfig {
	/// Short-term memory half-life.
	pub short_term_half_life: Duration,
	/// Minimum relevance before pruning.
	pub min_relevance: f64,
	/// Relevance boost when accessed.
	pub access_boost: f64,
}

impl Default for DecayConfig {
	fn default() -> Self {
		Self {
			short_term_half_life: Duration::from_secs(3600), // 1 hour
			min_relevance: 0.1,
			access_boost: 0.1,
		}
	}
}

/// In-memory store for memory items with indexing and decay.
pub struct MemoryStore {
	/// All memory items.
	items: Arc<RwLock<HashMap<MemoryId, MemoryItem>>>,
	/// Index by memory type.
	by_type: Arc<RwLock<HashMap<MemoryType, HashSet<MemoryId>>>>,
	/// Index by tag.
	by_tag: Arc<RwLock<HashMap<String, HashSet<MemoryId>>>>,
	/// Decay configuration.
	decay_config: DecayConfig,
	/// Resource budget.
	budget: ResourceBudget,
	/// Organization ID.
	org_id: OrgId,
}

impl MemoryStore {
	/// Creates a new memory store.
	pub fn new(org_id: OrgId, budget: ResourceBudget, decay_config: DecayConfig) -> Self {
		Self {
			items: Arc::new(RwLock::new(HashMap::new())),
			by_type: Arc::new(RwLock::new(HashMap::new())),
			by_tag: Arc::new(RwLock::new(HashMap::new())),
			decay_config,
			budget,
			org_id,
		}
	}

	/// Creates a new memory store with default configuration.
	pub fn with_defaults(org_id: OrgId) -> Self {
		Self::new(org_id, ResourceBudget::default(), DecayConfig::default())
	}

	/// Returns the organization ID.
	pub fn org_id(&self) -> OrgId {
		self.org_id
	}

	/// Returns the current item count.
	pub async fn len(&self) -> usize {
		self.items.read().await.len()
	}

	/// Returns true if the store is empty.
	pub async fn is_empty(&self) -> bool {
		self.items.read().await.is_empty()
	}

	/// Adds a memory item.
	pub async fn add(&self, item: MemoryItem) -> Result<MemoryId> {
		// Check budget - need to leave room for the new item
		let current_len = self.items.read().await.len();
		// Check if this is an update (item already exists) - no need to prune
		let is_update = self.items.read().await.contains_key(&item.id);
		if !is_update && current_len >= self.budget.max_memory_items {
			self.prune_to_budget_with_reserve(1).await?;
		}

		let id = item.id;

		// If item already exists, clean up old indexes first
		{
			let items = self.items.read().await;
			if let Some(old_item) = items.get(&id) {
				// Remove from old type index if type changed
				if old_item.memory_type != item.memory_type {
					let mut by_type = self.by_type.write().await;
					if let Some(ids) = by_type.get_mut(&old_item.memory_type) {
						ids.remove(&id);
					}
				}

				// Remove old tags from index
				let mut by_tag = self.by_tag.write().await;
				for old_tag in &old_item.tags {
					if !item.tags.contains(old_tag) {
						if let Some(ids) = by_tag.get_mut(old_tag) {
							ids.remove(&id);
						}
					}
				}
			}
		}

		// Index the item
		{
			let mut by_type = self.by_type.write().await;
			by_type.entry(item.memory_type).or_default().insert(id);
		}

		{
			let mut by_tag = self.by_tag.write().await;
			for tag in &item.tags {
				by_tag.entry(tag.clone()).or_default().insert(id);
			}
		}

		// Store the item
		{
			let mut items = self.items.write().await;
			items.insert(id, item);
		}

		Ok(id)
	}

	/// Retrieves a memory item by ID.
	pub async fn get(&self, id: &MemoryId) -> Option<MemoryItem> {
		let mut items = self.items.write().await;
		if let Some(item) = items.get_mut(id) {
			item.record_access();
			item.boost_relevance(self.decay_config.access_boost);
			Some(item.clone())
		} else {
			None
		}
	}

	/// Retrieves relevant memories for a query.
	pub async fn retrieve(&self, query: &MemoryQuery) -> Vec<MemoryItem> {
		let mut items = self.items.write().await;

		let mut results: Vec<_> = items
			.values()
			.filter(|item| self.matches_query(item, query))
			.cloned()
			.collect();

		// Sort by relevance
		results.sort_by(|a, b| {
			b.relevance_score
				.partial_cmp(&a.relevance_score)
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		// Update access timestamps for returned items
		for result in &results {
			if let Some(item) = items.get_mut(&result.id) {
				item.record_access();
				item.boost_relevance(self.decay_config.access_boost);
			}
		}

		// Limit results
		if let Some(max) = query.max_results {
			results.truncate(max);
		}

		results
	}

	/// Checks if an item matches a query.
	fn matches_query(&self, item: &MemoryItem, query: &MemoryQuery) -> bool {
		// Filter by type
		if let Some(ref mt) = query.memory_type {
			if item.memory_type != *mt {
				return false;
			}
		}

		// Filter by minimum relevance
		if let Some(min_rel) = query.min_relevance {
			if item.relevance_score < min_rel {
				return false;
			}
		}

		// Filter by tags (any match)
		if !query.tags.is_empty() {
			let has_matching_tag = query.tags.iter().any(|t| item.tags.contains(t));
			if !has_matching_tag {
				return false;
			}
		}

		// Filter by search text (simple substring match)
		if let Some(ref search) = query.search_text {
			let content_str = serde_json::to_string(&item.content).unwrap_or_default();
			if !content_str.to_lowercase().contains(&search.to_lowercase()) {
				return false;
			}
		}

		true
	}

	/// Applies decay to short-term memories.
	pub async fn apply_decay(&self) -> u32 {
		let mut items = self.items.write().await;
		let now = Utc::now();
		let mut decayed = 0;

		for item in items.values_mut() {
			if item.memory_type == MemoryType::ShortTerm {
				let age = now - item.last_accessed;
				let age_secs = age.num_seconds().max(0) as f64;
				let half_life_secs = self.decay_config.short_term_half_life.as_secs() as f64;

				let decay_factor = 0.5_f64.powf(age_secs / half_life_secs);
				item.decay_relevance(decay_factor);
				decayed += 1;
			}
		}

		// Prune items below threshold
		let min_relevance = self.decay_config.min_relevance;
		items.retain(|_, item| {
			item.memory_type == MemoryType::LongTerm || item.relevance_score >= min_relevance
		});

		decayed
	}

	/// Promotes a short-term memory to long-term.
	pub async fn promote(&self, id: &MemoryId) -> Result<()> {
		let mut items = self.items.write().await;

		if let Some(item) = items.get_mut(id) {
			// Update type index
			{
				let mut by_type = self.by_type.write().await;
				if let Some(set) = by_type.get_mut(&item.memory_type) {
					set.remove(id);
				}
				by_type.entry(MemoryType::LongTerm).or_default().insert(*id);
			}

			item.memory_type = MemoryType::LongTerm;
			item.relevance_score = 1.0;
			Ok(())
		} else {
			Err(ContextError::MemoryNotFound(id.to_string()))
		}
	}

	/// Removes a memory item.
	pub async fn remove(&self, id: &MemoryId) -> Result<MemoryItem> {
		let mut items = self.items.write().await;

		if let Some(item) = items.remove(id) {
			// Remove from type index
			{
				let mut by_type = self.by_type.write().await;
				if let Some(set) = by_type.get_mut(&item.memory_type) {
					set.remove(id);
				}
			}

			// Remove from tag indices
			{
				let mut by_tag = self.by_tag.write().await;
				for tag in &item.tags {
					if let Some(set) = by_tag.get_mut(tag) {
						set.remove(id);
					}
				}
			}

			Ok(item)
		} else {
			Err(ContextError::MemoryNotFound(id.to_string()))
		}
	}

	/// Prunes items to fit within budget, removing lowest relevance first.
	pub async fn prune_to_budget(&self) -> Result<u32> {
		self.prune_to_budget_with_reserve(0).await
	}

	/// Prunes items to fit within budget minus a reserve, removing lowest relevance first.
	async fn prune_to_budget_with_reserve(&self, reserve: usize) -> Result<u32> {
		let mut items = self.items.write().await;

		let target = self.budget.max_memory_items.saturating_sub(reserve);
		if items.len() <= target {
			return Ok(0);
		}

		let to_remove = items.len() - target;

		// Get items sorted by relevance (excluding long-term)
		let mut candidates: Vec<_> = items
			.iter()
			.filter(|(_, item)| item.memory_type != MemoryType::LongTerm)
			.map(|(id, item)| (*id, item.relevance_score))
			.collect();

		candidates.sort_by(|a, b| {
			a.1.partial_cmp(&b.1)
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		let ids_to_remove: Vec<_> = candidates.into_iter().take(to_remove).map(|(id, _)| id).collect();

		// Remove the items
		for id in &ids_to_remove {
			if let Some(item) = items.remove(id) {
				// Update indices
				{
					let mut by_type = self.by_type.write().await;
					if let Some(set) = by_type.get_mut(&item.memory_type) {
						set.remove(id);
					}
				}
				{
					let mut by_tag = self.by_tag.write().await;
					for tag in &item.tags {
						if let Some(set) = by_tag.get_mut(tag) {
							set.remove(id);
						}
					}
				}
			}
		}

		Ok(ids_to_remove.len() as u32)
	}

	/// Returns all items of a specific type.
	pub async fn get_by_type(&self, memory_type: MemoryType) -> Vec<MemoryItem> {
		let items = self.items.read().await;
		let by_type = self.by_type.read().await;

		if let Some(ids) = by_type.get(&memory_type) {
			ids.iter()
				.filter_map(|id| items.get(id).cloned())
				.collect()
		} else {
			Vec::new()
		}
	}

	/// Returns all items with a specific tag.
	pub async fn get_by_tag(&self, tag: &str) -> Vec<MemoryItem> {
		let items = self.items.read().await;
		let by_tag = self.by_tag.read().await;

		if let Some(ids) = by_tag.get(tag) {
			ids.iter()
				.filter_map(|id| items.get(id).cloned())
				.collect()
		} else {
			Vec::new()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_context_core::{MemoryContent, MemoryType};

	fn test_content() -> MemoryContent {
		MemoryContent::CodePattern {
			pattern: "test".to_string(),
			example: "example".to_string(),
			applicability: "tests".to_string(),
		}
	}

	#[tokio::test]
	async fn store_add_and_get() {
		let org_id = OrgId::new();
		let store = MemoryStore::with_defaults(org_id);

		let item = MemoryItem::new(
			org_id,
			MemoryType::ShortTerm,
			test_content(),
			loom_context_core::MemorySource::Manual,
		);
		let id = item.id;

		store.add(item).await.unwrap();
		assert_eq!(store.len().await, 1);

		let retrieved = store.get(&id).await.unwrap();
		assert_eq!(retrieved.id, id);
		assert_eq!(retrieved.access_count, 2); // Initial + get
	}

	#[tokio::test]
	async fn store_retrieve_with_query() {
		let org_id = OrgId::new();
		let store = MemoryStore::with_defaults(org_id);

		// Add items with different types and tags
		let item1 = MemoryItem::new(
			org_id,
			MemoryType::ShortTerm,
			test_content(),
			loom_context_core::MemorySource::Manual,
		)
		.with_tag("error");

		let item2 = MemoryItem::new(
			org_id,
			MemoryType::LongTerm,
			test_content(),
			loom_context_core::MemorySource::Manual,
		)
		.with_tag("pattern");

		store.add(item1).await.unwrap();
		store.add(item2).await.unwrap();

		// Query by type
		let query = MemoryQuery::new().with_type(MemoryType::LongTerm);
		let results = store.retrieve(&query).await;
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].memory_type, MemoryType::LongTerm);

		// Query by tag
		let query = MemoryQuery::new().with_tags(vec!["error".to_string()]);
		let results = store.retrieve(&query).await;
		assert_eq!(results.len(), 1);
	}

	#[tokio::test]
	async fn store_promotion() {
		let org_id = OrgId::new();
		let store = MemoryStore::with_defaults(org_id);

		let item = MemoryItem::new(
			org_id,
			MemoryType::ShortTerm,
			test_content(),
			loom_context_core::MemorySource::Manual,
		);
		let id = item.id;

		store.add(item).await.unwrap();
		store.promote(&id).await.unwrap();

		let retrieved = store.get(&id).await.unwrap();
		assert_eq!(retrieved.memory_type, MemoryType::LongTerm);
		assert!((retrieved.relevance_score - 1.0).abs() < 0.2); // Near 1.0 after boost
	}

	#[tokio::test]
	async fn store_prune_to_budget() {
		let org_id = OrgId::new();
		let budget = ResourceBudget::new().with_max_memory_items(3);
		let store = MemoryStore::new(org_id, budget, DecayConfig::default());

		// Add 5 items
		for i in 0..5 {
			let mut item = MemoryItem::new(
				org_id,
				MemoryType::ShortTerm,
				test_content(),
				loom_context_core::MemorySource::Manual,
			);
			item.relevance_score = i as f64 / 10.0;
			store.add(item).await.unwrap();
		}

		// Should have pruned to 3
		assert!(store.len().await <= 3);
	}

	#[tokio::test]
	async fn store_remove() {
		let org_id = OrgId::new();
		let store = MemoryStore::with_defaults(org_id);

		let item = MemoryItem::new(
			org_id,
			MemoryType::ShortTerm,
			test_content(),
			loom_context_core::MemorySource::Manual,
		);
		let id = item.id;

		store.add(item).await.unwrap();
		assert_eq!(store.len().await, 1);

		store.remove(&id).await.unwrap();
		assert_eq!(store.len().await, 0);
	}
}
