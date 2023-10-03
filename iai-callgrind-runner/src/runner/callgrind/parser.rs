use std::fmt::Display;
use std::str::FromStr;

use log::{trace, warn};
use serde::{Deserialize, Serialize};

use super::CallgrindOutput;
use crate::error::Result;

pub trait CallgrindParser {
    type Output;

    fn parse(self, output: &CallgrindOutput) -> Result<Self::Output>
    where
        Self: std::marker::Sized;
}

// TODO: Use CamelCase for sysCount etc.
// TODO: Add derived event types like Cycles, L1Hits etc.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventType,
    pub cost: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Costs(Vec<Event>);

impl Costs {
    pub fn with_event_types(types: &[EventType]) -> Self {
        Self(types.iter().map(|t| Event { kind: *t, cost: 0 }).collect())
    }

    pub fn add_iter_str<I, T>(&mut self, iter: T)
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        // From the documentation of the callgrind format:
        // > If a cost line specifies less event counts than given in the "events" line, the
        // > rest is assumed to be zero.
        for (event, cost) in self.0.iter_mut().zip(iter.into_iter()) {
            event.cost += cost.as_ref().parse::<u64>().unwrap();
        }
    }
    pub fn add(&mut self, other: &Self) {
        for (event, cost) in self
            .0
            .iter_mut()
            .zip(other.0.iter().map(|event| event.cost))
        {
            event.cost += cost;
        }
    }

    pub fn event_by_index(&self, index: usize) -> Option<&Event> {
        self.0.get(index)
    }

    pub fn event_by_type(&self, kind: EventType) -> Option<&Event> {
        self.0.iter().find(|e| e.kind == kind)
    }

    pub fn cost_by_index(&self, index: usize) -> Option<u64> {
        self.0.get(index).map(|e| e.cost)
    }

    pub fn cost_by_type(&self, kind: EventType) -> Option<u64> {
        self.0.iter().find(|e| e.kind == kind).map(|e| e.cost)
    }
}

impl Default for Costs {
    fn default() -> Self {
        Self(vec![Event {
            kind: EventType::Ir,
            cost: 0,
        }])
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
                .map(|s| Event {
                    kind: EventType::from(s),
                    cost: 0,
                })
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PositionsMode {
    Instr,
    Line,
    InstrLine,
}

impl PositionsMode {
    pub fn from_positions_line(line: &str) -> Option<Self> {
        Self::from_str(line.strip_prefix("positions:")?).ok()
    }
}

impl Default for PositionsMode {
    fn default() -> Self {
        Self::Line
    }
}

impl FromStr for PositionsMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut instr = false;
        let mut line = false;
        for split in s.trim().split_ascii_whitespace() {
            match split.to_lowercase().as_str() {
                "instr" | "addr" => instr = true,
                "line" => line = true,
                _ => return Err(format!("Invalid positions mode: '{split}'")),
            }
        }
        let mode = match (instr, line) {
            (true, true) => Self::InstrLine,
            (true, false) => Self::Instr,
            (false, true | false) => Self::Line,
        };
        std::result::Result::Ok(mode)
    }
}

pub struct CallgrindConfig {
    pub costs_prototype: Costs,
    pub positions_mode: PositionsMode,
}

pub fn parse_header(
    iter: &mut impl Iterator<Item = String>,
) -> std::result::Result<CallgrindConfig, String> {
    if !iter
        .by_ref()
        .find(|l| !l.trim().is_empty())
        .ok_or("Empty file")?
        .contains("callgrind format")
    {
        warn!("Missing file format specifier. Assuming callgrind format.");
    };

    let mut positions_mode: Option<PositionsMode> = None;
    let mut costs_prototype: Option<Costs> = None;

    for line in iter {
        if line.is_empty() || line.starts_with('#') {
            // skip empty lines or comments
            continue;
        }
        match line.split_once(':').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("version", value)) if value != "1" => {
                return Err(format!(
                    "Version mismatch: Requires version '1' but was '{value}'"
                ));
            }
            Some(("positions", mode)) => {
                positions_mode = Some(PositionsMode::from_str(mode)?);
                trace!("Using positions mode: '{:?}'", positions_mode);
            }
            // The events line is the last line in the header which is mandatory (according to
            // the source code of callgrind_annotate). The summary line is usually the last line
            // but it is only optional. So, we break out of the loop here and stop the parsing.
            Some(("events", mode)) => {
                trace!("Using events from line: '{line}'");
                costs_prototype = Some(mode.split_ascii_whitespace().collect::<Costs>());
                break;
            }
            // None is actually a malformed header line we just ignore here
            None | Some(_) => {
                continue;
            }
        }
    }

    Ok(CallgrindConfig {
        costs_prototype: costs_prototype
            .ok_or_else(|| "Header field 'events' must be present".to_owned())?,
        positions_mode: positions_mode.unwrap_or_default(),
    })
}
