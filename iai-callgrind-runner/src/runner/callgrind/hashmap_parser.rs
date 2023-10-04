use std::collections::HashMap;

use log::trace;
use serde::{Deserialize, Serialize};

use super::parser::{parse_header, CallgrindParser, Costs, EventType, PositionsMode};
use super::{CallgrindOutput, Sentinel};
use crate::error::{IaiCallgrindError, Result};

type ErrorMessageResult<T> = std::result::Result<T, String>;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallgrindMap {
    pub map: HashMap<Id, Record>,
    pub sentinel: Option<Sentinel>,
    pub sentinel_key: Option<Id>,
}

impl CallgrindMap {
    fn insert_record(&mut self, record: TemporaryRecord) {
        let func = record.func.expect("A record must have an fn entry");
        assert!(!func.is_empty(), "Expect the function to be not empty.");

        let key = Id { func };
        let value = Record {
            file: record.fl,
            inclusive_costs: record.inclusive_costs,
            self_costs: record.self_costs,
            ob: record.ob,
            members: record.members,
        };

        if self
            .sentinel
            .as_ref()
            .map_or(false, |sentinel| sentinel.matches(&key.func))
        {
            trace!("Found sentinel: {}", key.func);
            self.sentinel_key = Some(key.clone());
        }

        self.map.insert(key, value);
    }

    pub fn get_key_value(&self, k: &Id) -> Option<(&Id, &Record)> {
        self.map.get_key_value(k)
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &Record)> {
        self.map.iter()
    }

    fn new(sentinel: Option<&Sentinel>) -> CallgrindMap {
        Self {
            sentinel: sentinel.cloned(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashMapParser {
    pub sentinel: Option<Sentinel>,
}

impl CallgrindParser for HashMapParser {
    type Output = CallgrindMap;

    fn parse(self, output: &CallgrindOutput) -> Result<Self::Output> {
        LinesParser::default()
            .parse(CallgrindMap::new(self.sentinel.as_ref()), output.lines()?)
            .map_err(|message| IaiCallgrindError::ParseError((output.path.clone(), message)))
    }
}

/// The `TemporaryRecord` is used to collect all information until we can construct the key/value
/// pair for the hash map
#[derive(Debug, Default)]
struct TemporaryRecord {
    // fn
    func: Option<String>,
    ob: Option<String>,
    fl: Option<String>,
    inclusive_costs: Costs,
    self_costs: Costs,
    members: Vec<RecordMember>,
}

impl TemporaryRecord {
    pub fn from_prototype(costs: &Costs) -> Self {
        Self {
            inclusive_costs: costs.clone(),
            self_costs: costs.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Id {
    pub func: String,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineRecord {
    pub file: Option<String>,
    pub fi: Option<String>,
    pub fe: Option<String>,
    pub costs: Costs,
}

impl InlineRecord {
    pub fn from_prototype(costs: &Costs) -> Self {
        Self {
            costs: costs.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfnRecord {
    pub file: Option<String>,
    // a cfn line must be present
    pub cfn: String,
    pub cob: Option<String>,
    // and cfl
    pub cfi: Option<String>,
    // doesn't this depend on the PositionMode??
    pub calls: [u64; 2],
    pub costs: Costs,
}

impl CfnRecord {
    pub fn from_prototype(costs: &Costs) -> Self {
        Self {
            costs: costs.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub file: Option<String>,
    pub inclusive_costs: Costs,
    pub self_costs: Costs,
    pub ob: Option<String>,
    pub members: Vec<RecordMember>,
}

impl Record {
    pub fn from_prototype(costs: &Costs) -> Self {
        Self {
            inclusive_costs: costs.clone(),
            self_costs: costs.clone(),
            ..Default::default()
        }
    }

    pub fn with_event_types(types: &[EventType]) -> Self {
        let costs = Costs::with_event_types(types);
        Self {
            inclusive_costs: costs.clone(),
            self_costs: costs,
            ..Default::default()
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordMember {
    Cfn(CfnRecord),
    Inline(InlineRecord),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum State {
    Header,
    Record,
    CfnRecord,
    InlineRecord,
    CostLine,
    None,
    Footer,
}

type Split<'line> = Option<(&'line str, &'line str)>;

struct LinesParser {
    positions_mode: PositionsMode,
    costs_prototype: Costs,
    record: Option<TemporaryRecord>,
    cfn_record: Option<CfnRecord>,
    inline_record: Option<InlineRecord>,
    current_state: State,
    // The state before entering the cfn record
    old_state: Option<State>,
    target: Option<(String, String)>,
}

impl Default for LinesParser {
    fn default() -> Self {
        Self {
            positions_mode: PositionsMode::default(),
            costs_prototype: Costs::default(),
            record: Option::default(),
            cfn_record: Option::default(),
            inline_record: Option::default(),
            current_state: State::Header,
            old_state: Option::default(),
            target: Option::default(),
        }
    }
}

impl LinesParser {
    fn reset(&mut self) {
        self.record = None;
        self.cfn_record = None;
        self.inline_record = None;
        self.current_state = State::None;
        self.old_state = None;
        self.target = None;
    }

    fn set_state(&mut self, new_state: State) {
        self.current_state = new_state;
    }

    /// Used to save the state from before entering the [`State::CfnRecord`]
    fn save_cfn_state(&mut self) {
        if self.current_state != State::CfnRecord {
            self.old_state = Some(self.current_state);
        }
    }

    /// Used to restore the state saved with [`LinesParser::restore_cfn_state`]
    fn restore_cfn_state(&mut self) {
        self.current_state = self.old_state.expect("A saved state");
    }

    fn parse<I>(
        &mut self,
        mut callgrind_map: CallgrindMap,
        iter: I,
    ) -> std::result::Result<CallgrindMap, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut iter = iter.into_iter();
        // After calling this method, there might still be lines left in header format (like
        // `summary` or `totals`)
        let config = parse_header(&mut iter)?;
        self.positions_mode = config.positions_mode;
        self.costs_prototype = config.costs_prototype;

        for line in iter {
            if line.is_empty() && self.current_state != State::Header {
                if let Some(mut record) = self.record.take() {
                    if let Some(inline_record) = self.inline_record.take() {
                        record.members.push(RecordMember::Inline(inline_record));
                    }
                    callgrind_map.insert_record(record);
                }
                self.reset();
            } else if self.current_state == State::Footer {
                break;
            } else {
                self.handle_state(&line, line.split_once('='))?;
            }
        }

        if let Some(mut record) = self.record.take() {
            if let Some(inline_record) = self.inline_record.take() {
                record.members.push(RecordMember::Inline(inline_record));
            }
            callgrind_map.insert_record(record);
        }

        Ok(callgrind_map)
    }

    fn handle_state(&mut self, line: &str, split: Split) -> ErrorMessageResult<()> {
        match self.current_state {
            State::Header => self.handle_header_state(line, split),
            State::None => self.handle_none_state(line, split),
            State::Record => self.handle_record_state(line, split),
            State::CfnRecord => self.handle_cfn_record_state(line, split),
            State::InlineRecord => self.handle_inline_record_state(line, split),
            State::CostLine => self.handle_cost_line_state(line),
            State::Footer => Ok(()),
        }
    }

    fn handle_header_state(&mut self, line: &str, split: Split) -> ErrorMessageResult<()> {
        if split.is_some() {
            self.handle_record_state(line, split)
        } else {
            Ok(())
        }
    }

    fn handle_none_state(&mut self, line: &str, split: Split) -> ErrorMessageResult<()> {
        if line.starts_with("totals:") {
            self.current_state = State::Footer;
            Ok(())
        } else {
            self.handle_record_state(line, split)
        }
    }

    fn handle_record_state(&mut self, line: &str, split: Split) -> ErrorMessageResult<()> {
        match split {
            Some((key, value)) if key == "ob" => {
                let record = self
                    .record
                    .get_or_insert(TemporaryRecord::from_prototype(&self.costs_prototype));
                record.ob = Some(value.to_owned());
                self.target = Some((key.to_owned(), value.to_owned()));
                self.set_state(State::Record);
            }
            Some((key, value)) if key == "fl" => {
                let record = self
                    .record
                    .get_or_insert(TemporaryRecord::from_prototype(&self.costs_prototype));
                record.fl = Some(value.to_owned());
                self.target = Some((key.to_owned(), value.to_owned()));
                self.set_state(State::Record);
            }
            Some((key, value)) if key == "fn" => {
                let record = self
                    .record
                    .get_or_insert(TemporaryRecord::from_prototype(&self.costs_prototype));
                record.func = Some(value.to_owned());
                self.set_state(State::Record);
            }
            Some(_) => return self.handle_inline_record_state(line, split),
            None => return self.handle_cost_line_state(line),
        }

        Ok(())
    }

    fn handle_inline_record_state(&mut self, line: &str, split: Split) -> ErrorMessageResult<()> {
        match split {
            Some((key, value)) if key == "fi" => {
                let record = self
                    .record
                    .as_mut()
                    .expect("A record must be present at this point");
                if let Some(in_rec) = self.inline_record.take() {
                    record.members.push(RecordMember::Inline(in_rec));
                }
                self.inline_record = Some(InlineRecord {
                    fi: Some(value.to_owned()),
                    file: record.fl.clone(),
                    costs: self.costs_prototype.clone(),
                    ..Default::default()
                });
                self.target = Some((key.to_owned(), value.to_owned()));
                self.set_state(State::InlineRecord);
            }
            Some((key, value)) if key == "fe" => {
                let record = self
                    .record
                    .as_mut()
                    .expect("A record must be present at this point");
                if let Some(in_rec) = self.inline_record.take() {
                    record.members.push(RecordMember::Inline(in_rec));
                }
                match record.fl.as_ref() {
                    Some(file) if value == file => {
                        // This is a jump back to the original file so we can treat the
                        // following lines as if they were the record itself
                        self.set_state(State::Record);
                    }
                    None | Some(_) => {
                        self.inline_record = Some(InlineRecord {
                            fe: Some(value.to_owned()),
                            file: record.fl.clone(),
                            costs: self.costs_prototype.clone(),
                            ..Default::default()
                        });
                        self.set_state(State::InlineRecord);
                    }
                }
                self.target = Some((key.to_owned(), value.to_owned()));
            }
            Some(_) => return self.handle_cfn_record_state(line, split),
            None => return self.handle_cost_line_state(line),
        }

        Ok(())
    }

    fn handle_cfn_record_state(&mut self, line: &str, split: Split) -> ErrorMessageResult<()> {
        match split {
            Some(("cob", value)) => {
                let cfn_record = self
                    .cfn_record
                    .get_or_insert(CfnRecord::from_prototype(&self.costs_prototype));
                cfn_record.cob = Some(value.to_owned());
                self.save_cfn_state();
                self.set_state(State::CfnRecord);
            }
            // `cfi` and `cfl` are the same, they are just written differently because of historical
            // reasons
            Some(("cfi" | "cfl", value)) => {
                let cfn_record = self
                    .cfn_record
                    .get_or_insert(CfnRecord::from_prototype(&self.costs_prototype));
                cfn_record.cfi = Some(value.to_owned());
                self.save_cfn_state();
                self.set_state(State::CfnRecord);
            }
            Some(("cfn", value)) => {
                let cfn_record = self
                    .cfn_record
                    .get_or_insert(CfnRecord::from_prototype(&self.costs_prototype));
                cfn_record.cfn = value.to_owned();
                self.save_cfn_state();
                self.set_state(State::CfnRecord);
            }
            Some(("calls", value)) => {
                let cfn_record = self
                    .cfn_record
                    .get_or_insert(CfnRecord::from_prototype(&self.costs_prototype));
                for (index, count) in value
                    .split_ascii_whitespace()
                    .map(|s| s.parse::<u64>().expect("Parsing number should be ok"))
                    .enumerate()
                {
                    // TODO: OUT OF BOUNDS IF PositionMode IS InstrLine
                    cfn_record.calls[index] = count;
                }

                // There must be a cost line directly after a `calls` line, so we can directly set
                // the CostLine state
                self.save_cfn_state();
                self.set_state(State::CostLine);
            }
            Some(_) => return self.handle_unknown_state(line, &split),
            None => return self.handle_cost_line_state(line),
        }

        Ok(())
    }

    // Doesn't set a state by itself so the next handled state is the state before ending up here
    fn handle_unknown_state(&mut self, line: &str, split: &Split) -> ErrorMessageResult<()> {
        if split.is_some() {
            trace!("Found unknown specification: {}. Skipping it ...", line);
            Ok(())
        } else {
            self.handle_cost_line_state(line)
        }
    }

    // keep the method's return value in line with the other methods
    #[allow(clippy::unnecessary_wraps)]
    fn handle_cost_line_state(&mut self, line: &str) -> ErrorMessageResult<()> {
        // We check if it is a line starting with a digit. If not, it is a misinterpretation of the
        // callgrind format so we panic here.
        assert!(
            line.starts_with(|c: char| c.is_ascii_digit()),
            "Costline must start with a digit"
        );

        let mut costs = self.costs_prototype.clone();
        costs.add_iter_str(line
                        .split_ascii_whitespace()
                        // skip the first number which is just the line number or instr number or
                        // in case of `instr line` skip 2
                        .skip(if self.positions_mode == PositionsMode::InstrLine { 2 } else { 1 }));

        let record = self
            .record
            .as_mut()
            .expect("A record must be present at this state");

        // A cfn record takes precedence over an inline record (=fe/fi) and an inline record takes
        // precedence over a record.
        if let Some(mut cfn_record) = self.cfn_record.take() {
            assert!(
                !cfn_record.cfn.is_empty(),
                "A cfn record must have an cfn entry"
            );

            cfn_record.costs = costs;
            record.inclusive_costs.add(&cfn_record.costs);

            cfn_record.file = match cfn_record.cfi.as_deref() {
                None | Some("???") => match cfn_record.cob.as_deref() {
                    None | Some("???") => self.target.as_ref().map(|(_, v)| v.clone()),
                    Some(value) => Some(value.to_owned()),
                },
                Some(value) => Some(value.to_owned()),
            };

            record.members.push(RecordMember::Cfn(cfn_record));

            // A cfn record has exactly 1 cost line, so we can restore the state from before the cfn
            // state here
            self.restore_cfn_state();

            // An inline record can have multiple cost lines so we cannot end an `InlineRecord`
            // here. Only another inline record can end an inlinerecord.
        } else if let Some(inline_record) = self.inline_record.as_mut() {
            inline_record.costs.add(&costs);
            record.inclusive_costs.add(&costs);

            self.set_state(State::InlineRecord);
            // Much like inline records, a Record can have mulitple cost lines.
        } else {
            record.inclusive_costs.add(&costs);
            record.self_costs.add(&costs);

            self.set_state(State::Record);
        }

        Ok(())
    }
}
