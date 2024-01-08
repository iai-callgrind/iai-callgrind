use anyhow::{anyhow, Result};
use indexmap::map::Iter;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Costs<K: Hash + Eq>(pub IndexMap<K, u64>);

impl<K: Hash + Eq + Display + Clone> Costs<K> {
    // The order matters. The index is derived from the insertion order
    pub fn with_event_kinds<T>(kinds: T) -> Self
    where
        T: IntoIterator<Item = (K, u64)>,
    {
        Self(kinds.into_iter().collect())
    }

    pub fn add_iter_str<I, T>(&mut self, iter: T)
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        // From the documentation of the callgrind format:
        // > If a cost line specifies less event counts than given in the "events" line, the
        // > rest is assumed to be zero.
        for ((_, old), cost) in self.0.iter_mut().zip(iter.into_iter()) {
            *old += cost.as_ref().parse::<u64>().unwrap();
        }
    }

    pub fn add(&mut self, other: &Self) {
        for ((_, old), cost) in self.0.iter_mut().zip(other.0.iter().map(|(_, c)| c)) {
            *old += cost;
        }
    }

    /// Return the cost of the event at index (of insertion order) if present
    ///
    /// This operation is O(1)
    pub fn cost_by_index(&self, index: usize) -> Option<u64> {
        self.0.get_index(index).map(|(_, c)| *c)
    }

    /// Return the cost of the [`EventKind`] if present
    ///
    /// This operation is O(1)
    pub fn cost_by_kind(&self, kind: &K) -> Option<u64> {
        self.0.get_key_value(kind).map(|(_, c)| *c)
    }

    pub fn try_cost_by_kind(&self, kind: &K) -> Result<u64> {
        self.cost_by_kind(kind)
            .ok_or_else(|| anyhow!("Missing event type '{kind}"))
    }

    pub fn event_kinds(&self) -> Vec<K> {
        self.0.iter().map(|(k, _)| k.clone()).collect()
    }

    pub fn event_kinds_union(&self, other: &Self) -> IndexSet<K> {
        let set = self.0.keys().collect::<IndexSet<_>>();
        let other_set = other.0.keys().collect::<IndexSet<_>>();
        set.union(&other_set).map(|s| (*s).clone()).collect()
    }

    pub fn iter(&self) -> Iter<'_, K, u64> {
        self.0.iter()
    }
}

pub trait Summarize: Hash + Eq + Clone {
    fn summarize(_: &mut Cow<Costs<Self>>) {}
}

impl Summarize for String {}

impl<'a, K: Hash + Eq + Display + Clone> IntoIterator for &'a Costs<K> {
    type Item = (&'a K, &'a u64);

    type IntoIter = Iter<'a, K, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<I, K: Hash + Eq + From<I>> FromIterator<I> for Costs<K> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        Self(
            iter.into_iter()
                .map(|s| (K::from(s), 0))
                .collect::<IndexMap<_, _>>(),
        )
    }
}
