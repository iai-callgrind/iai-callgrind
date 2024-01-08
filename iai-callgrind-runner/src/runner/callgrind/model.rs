//! This module includes all the structs to model the callgrind output

use anyhow::Result;
use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::hash::Hash;

use super::CacheSummary;
use crate::api::EventKind;
use crate::runner::costs::Summarize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calls {
    amount: u64,
    positions: Positions,
}

pub type Costs = crate::runner::costs::Costs<EventKind>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PositionType {
    Instr,
    Line,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Positions(IndexMap<PositionType, u64>);

impl Calls {
    pub fn from<I>(mut iter: impl Iterator<Item = I>, mut positions: Positions) -> Self
    where
        I: AsRef<str>,
    {
        let amount = iter.next().unwrap().as_ref().parse().unwrap();
        positions.set_iter_str(iter);
        Self { amount, positions }
    }
}

impl Costs {
    /// Calculate and add derived summary events (i.e. estimated cycles) in-place
    ///
    /// Additional calls to this function will overwrite the costs for derived summary events.
    ///
    /// # Errors
    ///
    /// If the necessary cache simulation events (when running callgrind with --cache-sim) were not
    /// present.
    pub fn make_summary(&mut self) -> Result<()> {
        let CacheSummary {
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        } = (&*self).try_into()?;

        self.0.insert(EventKind::L1hits, l1_hits);
        self.0.insert(EventKind::LLhits, l3_hits);
        self.0.insert(EventKind::RamHits, ram_hits);
        self.0.insert(EventKind::TotalRW, total_memory_rw);
        self.0.insert(EventKind::EstimatedCycles, cycles);

        Ok(())
    }

    /// Return true if costs are already summarized
    ///
    /// This method just probes for [`EventKind::EstimatedCycles`] to detect the summarized state.
    pub fn is_summarized(&self) -> bool {
        self.cost_by_kind(&EventKind::EstimatedCycles).is_some()
    }
}

impl Summarize for EventKind {
    fn summarize(costs: &mut Cow<Costs>) {
        if !costs.is_summarized() {
            let _ = costs.to_mut().make_summary();
        }
    }
}
impl Default for Costs {
    fn default() -> Self {
        Self(indexmap! {EventKind::Ir => 0})
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
