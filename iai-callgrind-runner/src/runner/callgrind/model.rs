//! This module includes all the structs to model the callgrind output

use std::fmt::Display;

use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};

// TODO: Use CamelCase for sysCount etc.
// TODO: Add derived event types like Cycles, L1Hits etc.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    // always on
    Ir,
    // --collect-systime
    sysCount,
    sysTime,
    sysCpuTime,
    // --collect-bus
    Ge,
    // --cache-sim
    Dr,
    Dw,
    I1mr,
    ILmr,
    D1mr,
    DLmr,
    D1mw,
    DLmw,
    // --branch-sim
    Bc,
    Bcm,
    Bi,
    Bim,
    // --simulate-wb
    ILdmr,
    DLdmr,
    DLdmw,
    // --cachuse
    AcCost1,
    AcCost2,
    SpLoss1,
    SpLoss2,
}

impl<T> From<T> for EventType
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "Ir" => Self::Ir,
            "Dr" => Self::Dr,
            "Dw" => Self::Dw,
            "I1mr" => Self::I1mr,
            "ILmr" => Self::ILmr,
            "D1mr" => Self::D1mr,
            "DLmr" => Self::DLmr,
            "D1mw" => Self::D1mw,
            "DLmw" => Self::DLmw,
            "sysCount" => Self::sysCount,
            "sysTime" => Self::sysTime,
            "sysCpuTime" => Self::sysCpuTime,
            "Ge" => Self::Ge,
            "Bc" => Self::Bc,
            "Bcm" => Self::Bcm,
            "Bi" => Self::Bi,
            "Bim" => Self::Bim,
            "ILdmr" => Self::ILdmr,
            "DLdmr" => Self::DLdmr,
            "DLdmw" => Self::DLdmw,
            "AcCost1" => Self::AcCost1,
            "AcCost2" => Self::AcCost2,
            "SpLoss1" => Self::SpLoss1,
            "SpLoss2" => Self::SpLoss2,
            unknown => unreachable!("Unknown event type: {unknown}"),
        }
    }
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Costs(IndexMap<EventType, u64>);

impl Costs {
    // The order matters. The index is derived from the insertion order
    pub fn with_event_types(types: &[EventType]) -> Self {
        Self(types.iter().map(|t| (*t, 0)).collect())
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

    /// Return the cost of the event at index (of insertion order)
    ///
    /// This operation is O(1)
    pub fn cost_by_index(&self, index: usize) -> Option<u64> {
        self.0.get_index(index).map(|(_, c)| *c)
    }

    /// Return the cost of the [`EventType`]
    ///
    /// This operation is O(1)
    pub fn cost_by_type(&self, kind: &EventType) -> Option<u64> {
        self.0.get_key_value(kind).map(|(_, c)| *c)
    }
}

impl Default for Costs {
    fn default() -> Self {
        Self(indexmap! {EventType::Ir => 0})
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
                .map(|s| (EventType::from(s), 0))
                .collect::<IndexMap<_, _>>(),
        )
    }
}
