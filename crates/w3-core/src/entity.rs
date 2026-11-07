//! Entity identity. IDs are positive integers, unique within a match and never
//! reused. A `BTreeSet` keeps iteration in stable ID order for hashing.

use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityId(u64);

impl EntityId {
    /// Zero is reserved as "no entity".
    pub const NONE: Self = Self(0);

    pub fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct EntityStore {
    next_id: u64,
    alive: BTreeSet<EntityId>,
}

impl EntityStore {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            alive: BTreeSet::new(),
        }
    }

    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.alive.insert(id);
        id
    }

    pub fn despawn(&mut self, id: EntityId) -> bool {
        self.alive.remove(&id)
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.alive.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.alive.len()
    }

    pub fn is_empty(&self) -> bool {
        self.alive.is_empty()
    }

    /// Stable ID order — never a hash-map iteration order.
    pub fn iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.alive.iter().copied()
    }

    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Snapshot restore only.
    pub(crate) fn restore(next_id: u64, ids: &[u64]) -> Self {
        let mut alive = BTreeSet::new();
        for &value in ids {
            if let Some(id) = EntityId::new(value) {
                alive.insert(id);
            }
        }
        Self { next_id, alive }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_reserved() {
        assert_eq!(EntityId::NONE, EntityId(0));
        assert_eq!(EntityId::new(0), None);
    }

    #[test]
    fn ids_are_never_reused() {
        let mut store = EntityStore::new();
        let a = store.spawn();
        let b = store.spawn();
        assert_eq!(a.value(), 1);
        assert_eq!(b.value(), 2);
        assert!(store.despawn(a));
        let c = store.spawn();
        assert_eq!(c.value(), 3);
        assert_ne!(a, c);
        assert_eq!(store.next_id(), 4);
    }

    #[test]
    fn iteration_is_in_stable_id_order() {
        let mut store = EntityStore::new();
        let a = store.spawn();
        let b = store.spawn();
        let c = store.spawn();
        let ids: Vec<_> = store.iter().collect();
        assert_eq!(ids, vec![a, b, c]);
        assert_eq!(ids, vec![EntityId(1), EntityId(2), EntityId(3)]);
    }
}
