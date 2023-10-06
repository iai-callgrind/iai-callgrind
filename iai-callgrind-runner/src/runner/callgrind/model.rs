//! This module includes all the structs to model the callgrind output

use std::fmt::Display;

use indexmap::{indexmap, IndexMap};
use serde::{Deserialize, Serialize};

use crate::api;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calls {
    amount: u64,
    positions: Positions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Costs(IndexMap<EventType, u64>);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    // always on
    Ir,
    // --collect-systime
    SysCount,
    SysTime,
    SysCpuTime,
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

    pub fn event_types(&self) -> Vec<EventType> {
        self.0.iter().map(|(k, _)| *k).collect()
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

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

impl From<api::EventType> for EventType {
    fn from(value: api::EventType) -> Self {
        match value {
            api::EventType::Ir => EventType::Ir,
            api::EventType::Dr => EventType::Dr,
            api::EventType::Dw => EventType::Dw,
            api::EventType::I1mr => EventType::I1mr,
            api::EventType::ILmr => EventType::ILmr,
            api::EventType::D1mr => EventType::D1mr,
            api::EventType::DLmr => EventType::DLmr,
            api::EventType::D1mw => EventType::D1mw,
            api::EventType::DLmw => EventType::DLmw,
            api::EventType::SysCount => EventType::SysCount,
            api::EventType::SysTime => EventType::SysTime,
            api::EventType::SysCpuTime => EventType::SysCpuTime,
            api::EventType::Ge => EventType::Ge,
            api::EventType::Bc => EventType::Bc,
            api::EventType::Bcm => EventType::Bcm,
            api::EventType::Bi => EventType::Bi,
            api::EventType::Bim => EventType::Bim,
            api::EventType::ILdmr => EventType::ILdmr,
            api::EventType::DLdmr => EventType::DLdmr,
            api::EventType::DLdmw => EventType::DLdmw,
            api::EventType::AcCost1 => EventType::AcCost1,
            api::EventType::AcCost2 => EventType::AcCost2,
            api::EventType::SpLoss1 => EventType::SpLoss1,
            api::EventType::SpLoss2 => EventType::SpLoss2,
        }
    }
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
            "sysCount" => Self::SysCount,
            "sysTime" => Self::SysTime,
            "sysCpuTime" => Self::SysCpuTime,
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
            unknown => panic!("Unknown event type: {unknown}"),
        }
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
