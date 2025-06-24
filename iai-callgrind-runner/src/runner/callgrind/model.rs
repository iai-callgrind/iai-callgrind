//! This module includes all the structs to model the callgrind output

use std::borrow::Cow;
use std::hash::Hash;

use anyhow::Result;
use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};

use super::CacheSummary;
use crate::api::EventKind;
use crate::runner::metrics::{Metric, Summarize};

pub type Metrics = crate::runner::metrics::Metrics<EventKind>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calls {
    amount: u64,
    positions: Positions,
}

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

impl Metrics {
    /// Calculate and add derived summary events (i.e. estimated cycles) in-place
    ///
    /// Additional calls to this function will overwrite the metrics for derived summary events.
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
            i1_miss_rate,
            d1_miss_rate,
            ll_miss_rate,
            lli_miss_rate,
            lld_miss_rate,
            l1_hit_rate,
            l3_hit_rate,
            ram_hit_rate,
        } = (&*self).try_into()?;

        self.insert(EventKind::L1hits, l1_hits);
        self.insert(EventKind::LLhits, l3_hits);
        self.insert(EventKind::RamHits, ram_hits);
        self.insert(EventKind::TotalRW, total_memory_rw);
        self.insert(EventKind::EstimatedCycles, cycles);
        self.insert(EventKind::I1MissRate, i1_miss_rate);
        self.insert(EventKind::D1MissRate, d1_miss_rate);
        self.insert(EventKind::LLiMissRate, lli_miss_rate);
        self.insert(EventKind::LLdMissRate, lld_miss_rate);
        self.insert(EventKind::LLMissRate, ll_miss_rate);
        self.insert(EventKind::L1HitRate, l1_hit_rate);
        self.insert(EventKind::LLHitRate, l3_hit_rate);
        self.insert(EventKind::RamHitRate, ram_hit_rate);

        Ok(())
    }

    /// Return true if costs are already summarized
    ///
    /// This method just probes for [`EventKind::EstimatedCycles`] to detect the summarized state.
    pub fn is_summarized(&self) -> bool {
        self.metric_by_kind(&EventKind::EstimatedCycles).is_some()
    }

    /// Return true if costs can be summarized
    ///
    /// This method probes for [`EventKind::I1mr`] which is present if callgrind was run with the
    /// cache simulation (`--cache-sim=yes`) enabled.
    pub fn can_summarize(&self) -> bool {
        self.metric_by_kind(&EventKind::I1mr).is_some()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self(indexmap! {EventKind::Ir => Metric::Int(0)})
    }
}

impl Summarize for EventKind {
    fn summarize(costs: &mut Cow<Metrics>) {
        if !costs.is_summarized() {
            let _ = costs.to_mut().make_summary();
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
