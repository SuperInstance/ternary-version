//! # ternary-version
//!
//! Version vectors with ternary comparison for distributed GPU state.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionOrder { Newer = 1, Concurrent = 0, Older = -1 }

#[derive(Debug, Clone)]
pub struct VersionVector {
    entries: HashMap<String, u64>,
}

impl VersionVector {
    pub fn new() -> Self { Self { entries: HashMap::new() } }

    pub fn increment(&mut self, node: &str) -> u64 {
        let v = self.entries.entry(node.into()).or_insert(0);
        *v += 1;
        *v
    }

    pub fn get(&self, node: &str) -> u64 { self.entries.get(node).copied().unwrap_or(0) }

    pub fn set(&mut self, node: &str, version: u64) {
        self.entries.insert(node.into(), version);
    }

    /// Compare two version vectors: +1 if self dominates, -1 if other dominates, 0 if concurrent.
    pub fn compare(&self, other: &VersionVector) -> VersionOrder {
        let all_keys: std::collections::HashSet<&String> =
            self.entries.keys().chain(other.entries.keys()).collect();

        let mut self_greater = false;
        let mut other_greater = false;

        for key in &all_keys {
            let a = self.entries.get(*key).copied().unwrap_or(0);
            let b = other.entries.get(*key).copied().unwrap_or(0);
            if a > b { self_greater = true; }
            if b > a { other_greater = true; }
        }

        if self_greater && !other_greater { VersionOrder::Newer }
        else if other_greater && !self_greater { VersionOrder::Older }
        else { VersionOrder::Concurrent }
    }

    /// Merge: take component-wise max.
    pub fn merge(&mut self, other: &VersionVector) {
        for (key, &val) in &other.entries {
            let current = self.entries.entry(key.clone()).or_insert(0);
            *current = (*current).max(val);
        }
    }

    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn node_count(&self) -> usize { self.entries.len() }
}

impl Default for VersionVector { fn default() -> Self { Self::new() } }

pub struct ConflictResolver {
    resolver_id: String,
    clock: VersionVector,
    resolved: u64,
}

impl ConflictResolver {
    pub fn new(id: &str) -> Self {
        Self { resolver_id: id.into(), clock: VersionVector::new(), resolved: 0 }
    }

    /// Resolve conflict between two versions. Returns merged vector.
    pub fn resolve(&mut self, a: &VersionVector, b: &VersionVector) -> VersionVector {
        let mut merged = a.clone();
        merged.merge(b);
        merged.increment(&self.resolver_id);
        self.clock.merge(&merged);
        self.resolved += 1;
        merged
    }

    pub fn resolved_count(&self) -> u64 { self.resolved }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment() {
        let mut vv = VersionVector::new();
        vv.increment("node_a");
        vv.increment("node_a");
        assert_eq!(vv.get("node_a"), 2);
    }

    #[test]
    fn test_newer() {
        let mut a = VersionVector::new();
        let b = VersionVector::new();
        a.set("x", 2);
        a.set("y", 1);
        assert_eq!(a.compare(&b), VersionOrder::Newer);
    }

    #[test]
    fn test_older() {
        let a = VersionVector::new();
        let mut b = VersionVector::new();
        b.set("x", 5);
        assert_eq!(a.compare(&b), VersionOrder::Older);
    }

    #[test]
    fn test_concurrent() {
        let mut a = VersionVector::new();
        let mut b = VersionVector::new();
        a.set("x", 2);
        b.set("y", 2);
        assert_eq!(a.compare(&b), VersionOrder::Concurrent);
    }

    #[test]
    fn test_merge() {
        let mut a = VersionVector::new();
        let mut b = VersionVector::new();
        a.set("x", 3);
        b.set("x", 5);
        b.set("y", 2);
        a.merge(&b);
        assert_eq!(a.get("x"), 5);
        assert_eq!(a.get("y"), 2);
    }

    #[test]
    fn test_equal() {
        let mut a = VersionVector::new();
        let mut b = VersionVector::new();
        a.set("x", 3);
        b.set("x", 3);
        assert_eq!(a.compare(&b), VersionOrder::Concurrent); // equal is concurrent
    }

    #[test]
    fn test_resolver() {
        let mut cr = ConflictResolver::new("resolver1");
        let mut a = VersionVector::new();
        let mut b = VersionVector::new();
        a.set("x", 2);
        b.set("x", 3);
        let merged = cr.resolve(&a, &b);
        assert_eq!(merged.get("x"), 3);
        assert_eq!(cr.resolved_count(), 1);
    }

    #[test]
    fn test_empty_compare() {
        let a = VersionVector::new();
        let b = VersionVector::new();
        assert_eq!(a.compare(&b), VersionOrder::Concurrent);
    }
}
