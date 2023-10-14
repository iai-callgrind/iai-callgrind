//! This module includes all the structs to model the callgrind output

use anyhow::{anyhow, Result};
use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};

use crate::api::EventKind;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calls {
    amount: u64,
    positions: Positions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Costs(IndexMap<EventKind, u64>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PositionType {
    Instr,
    Line,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Positions(IndexMap<PositionType, u64>);

impl Calls {
    pub fn from<I>(mut iter: impl Iterator<Item = I>, positions: &Positions) -> Self
    where
        I: AsRef<str>,
    {
        let amount = iter.next().unwrap().as_ref().parse().unwrap();
        let mut positions = positions.clone();
        positions.set_iter_str(iter);
        Self { amount, positions }
    }
}

impl Costs {
    // The order matters. The index is derived from the insertion order
    pub fn with_event_kinds(kinds: &[EventKind]) -> Self {
        Self(kinds.iter().map(|t| (*t, 0)).collect())
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

    /// Return the cost of the [`EventType`] if present
    ///
    /// This operation is O(1)
    pub fn cost_by_kind(&self, kind: &EventKind) -> Option<u64> {
        self.0.get_key_value(kind).map(|(_, c)| *c)
    }

    pub fn try_cost_by_kind(&self, kind: &EventKind) -> Result<u64> {
        self.cost_by_kind(kind)
            .ok_or_else(|| anyhow!("Missing event type '{kind}"))
    }

    pub fn event_kinds(&self) -> Vec<EventKind> {
        self.0.iter().map(|(k, _)| *k).collect()
    }

    /// Calculate summary events and estimated cycles in-place
    ///
    /// # Panics
    ///
    /// If the necessary cache simulation events (when running callgrind with --cache-sim) were not
    /// present.
    pub fn make_summary(&mut self) -> Result<()> {
        //         0   1  2    3    4    5    6    7    8
        // events: Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let instructions = self.try_cost_by_kind(&EventKind::Ir)?;
        let total_data_cache_reads = self.try_cost_by_kind(&EventKind::Dr)?;
        let total_data_cache_writes = self.try_cost_by_kind(&EventKind::Dw)?;
        let l1_instructions_cache_read_misses = self.try_cost_by_kind(&EventKind::I1mr)?;
        let l1_data_cache_read_misses = self.try_cost_by_kind(&EventKind::D1mr)?;
        let l1_data_cache_write_misses = self.try_cost_by_kind(&EventKind::D1mw)?;
        let l3_instructions_cache_read_misses = self.try_cost_by_kind(&EventKind::ILmr)?;
        let l3_data_cache_read_misses = self.try_cost_by_kind(&EventKind::DLmr)?;
        let l3_data_cache_write_misses = self.try_cost_by_kind(&EventKind::DLmw)?;

        let ram_hits = l3_instructions_cache_read_misses
            + l3_data_cache_read_misses
            + l3_data_cache_write_misses;
        let l1_data_accesses = l1_data_cache_read_misses + l1_data_cache_write_misses;
        let l1_miss = l1_instructions_cache_read_misses + l1_data_accesses;
        let l3_accesses = l1_miss;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw = instructions + total_data_cache_reads + total_data_cache_writes;
        let l1_hits = total_memory_rw - ram_hits - l3_hits;

        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        let cycles = l1_hits + (5 * l3_hits) + (35 * ram_hits);

        self.0.insert(EventKind::L1hits, l1_hits);
        self.0.insert(EventKind::LLhits, l3_hits);
        self.0.insert(EventKind::RamHits, ram_hits);
        self.0.insert(EventKind::TotalRW, total_memory_rw);
        self.0.insert(EventKind::EstimatedCycles, cycles);

        Ok(())
    }
}

impl Default for Costs {
    fn default() -> Self {
        Self(indexmap! {EventKind::Ir => 0})
    }
}

impl<I> FromIterator<I> for Costs
where
    I: AsRef<str>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        Self(
            iter.into_iter()
                .map(|s| (EventKind::from(s), 0))
                .collect::<IndexMap<_, _>>(),
        )
    }
}

impl<T> From<T> for PositionType
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        let value = value.as_ref();
        // "addr" is taken from the callgrind_annotate script although not officially documented
        match value.to_lowercase().as_str() {
            "instr" | "addr" => Self::Instr,
            "line" => Self::Line,
            _ => panic!("Unknown positions type: '{value}"),
        }
    }
}

impl Positions {
    pub fn set_iter_str<I, T>(&mut self, iter: T)
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        for ((_, old), pos) in self.0.iter_mut().zip(iter.into_iter()) {
            let pos = pos.as_ref();
            *old = if let Some(hex) = pos.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).unwrap()
            } else {
                pos.parse::<u64>().unwrap()
            }
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Positions {
    fn default() -> Self {
        Self(indexmap! {PositionType::Line => 0})
    }
}

impl<I> FromIterator<I> for Positions
where
    I: AsRef<str>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        Self(
            iter.into_iter()
                .map(|p| (PositionType::from(p), 0))
                .collect::<IndexMap<_, _>>(),
        )
    }
}
