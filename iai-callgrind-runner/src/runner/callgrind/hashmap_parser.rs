use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use log::{trace, warn};

use super::{CallgrindParser, Sentinel};
use crate::error::Result;
use crate::runner::callgrind::PositionsMode;

#[derive(Debug, Default)]
pub struct HashMapParser {
    pub map: HashMap<Id, Record>,
    pub sentinel: Option<Sentinel>,
    pub resolved_sentinel: Option<Id>,
}

impl HashMapParser {
    fn insert_record(&mut self, record: TemporaryRecord) {
        let func = record.func.expect("A record must have an fn entry");
        assert!(!func.is_empty(), "Expect the function to be not empty.");

        let key = Id { func };
        let value = Record {
            file: record.fl,
            inclusive_costs: record.inclusive_costs,
            self_costs: record.self_costs,
            ob: record.ob,
            cfns: record.cfns,
            inlines: record.inlines,
        };

        if self
            .sentinel
            .as_ref()
            .map_or(false, |sentinel| sentinel.matches(&key.func))
        {
            trace!("Found sentinel: {}", key.func);
            self.resolved_sentinel = Some(key.clone());
        }

        self.map.insert(key, value);
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
    inclusive_costs: [u64; 9],
    self_costs: [u64; 9],
    cfns: Vec<CfnRecord>,
    // fi and fe if the target of an fe entry is not the func itself
    inlines: Vec<InlineRecord>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Id {
    pub func: String,
}

#[derive(Debug, Default)]
pub struct InlineRecord {
    pub file: Option<String>,
    pub fi: Option<String>,
    pub fe: Option<String>,
    pub costs: [u64; 9],
}

#[derive(Debug, Default)]
pub struct CfnRecord {
    pub file: Option<String>,
    // a cfn line must be present
    pub cfn: String,
    pub cob: Option<String>,
    // and cfl
    pub cfi: Option<String>,
    // doesn't this depend on the PositionMode??
    pub calls: [u64; 2],
    pub costs: [u64; 9],
}

#[derive(Debug, Default)]
pub struct Record {
    pub file: Option<String>,
    pub inclusive_costs: [u64; 9],
    pub self_costs: [u64; 9],
    pub ob: Option<String>,
    pub cfns: Vec<CfnRecord>,
    pub inlines: Vec<InlineRecord>,
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

struct LinesParser {
    positions_mode: PositionsMode,
    record: Option<TemporaryRecord>,
    cfn_record: Option<CfnRecord>,
    inline_record: Option<InlineRecord>,
    current_state: State,
    // The state before entering the cfn record
    old_state: Option<State>,
    target: Option<(String, String)>,
}

type Split<'line> = Option<(&'line str, &'line str)>;

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

    fn parse<I>(&mut self, hash_map_parser: &mut HashMapParser, mut iter: I)
    where
        I: Iterator<Item = String>,
    {
        self.positions_mode = iter
            .find_map(|line| PositionsMode::from_positions_line(&line))
            .expect("Callgrind output line with mode for positions");
        trace!("Using positions mode: {:?}", self.positions_mode);

        for line in iter {
            if line.is_empty() && self.current_state != State::Header {
                if let Some(record) = self.record.take() {
                    hash_map_parser.insert_record(record);
                }
                self.reset();
            } else if self.current_state == State::Footer {
                break;
            } else {
                self.handle_state(&line, line.split_once('='));
            }
        }

        if let Some(record) = self.record.take() {
            hash_map_parser.insert_record(record);
        }
    }

    fn handle_state(&mut self, line: &str, split: Split) {
        match self.current_state {
            State::Header => self.handle_header_state(line, split),
            State::None => self.handle_none_state(line, split),
            State::Record => self.handle_record_state(line, split),
            State::CfnRecord => self.handle_cfn_record_state(line, split),
            State::InlineRecord => self.handle_inline_record_state(line, split),
            State::CostLine => self.handle_cost_line_state(line),
            State::Footer => {}
        }
    }

    fn handle_header_state(&mut self, line: &str, split: Split) {
        if split.is_some() {
            self.handle_record_state(line, split);
        }
    }

    fn handle_none_state(&mut self, line: &str, split: Split) {
        if line.starts_with("totals:") {
            self.current_state = State::Footer;
        } else {
            self.handle_record_state(line, split);
        }
    }

    fn handle_record_state(&mut self, line: &str, split: Split) {
        match split {
            Some((key, value)) if key == "ob" => {
                let record = self.record.get_or_insert(TemporaryRecord::default());
                record.ob = Some(value.to_owned());
                self.target = Some((key.to_owned(), value.to_owned()));
                self.set_state(State::Record);
            }
            Some((key, value)) if key == "fl" => {
                let record = self.record.get_or_insert(TemporaryRecord::default());
                record.fl = Some(value.to_owned());
                self.target = Some((key.to_owned(), value.to_owned()));
                self.set_state(State::Record);
            }
            Some((key, value)) if key == "fn" => {
                let record = self.record.get_or_insert(TemporaryRecord::default());
                record.func = Some(value.to_owned());
                self.set_state(State::Record);
            }
            Some(_) => self.handle_inline_record_state(line, split),
            None => self.handle_cost_line_state(line),
        }
    }

    fn handle_inline_record_state(&mut self, line: &str, split: Split) {
        match split {
            Some((key, value)) if key == "fi" => {
                let record = self
                    .record
                    .as_mut()
                    .expect("A record must be present at this point");
                if let Some(in_rec) = self.inline_record.take() {
                    record.inlines.push(in_rec);
                }
                self.inline_record = Some(InlineRecord {
                    fi: Some(value.to_owned()),
                    file: record.fl.clone(),
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
                    record.inlines.push(in_rec);
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
                            ..Default::default()
                        });
                        self.set_state(State::InlineRecord);
                    }
                }
                self.target = Some((key.to_owned(), value.to_owned()));
            }
            Some(_) => self.handle_cfn_record_state(line, split),
            None => self.handle_cost_line_state(line),
        }
    }

    fn handle_cfn_record_state(&mut self, line: &str, split: Split) {
        match split {
            Some(("cob", value)) => {
                let cfn_record = self.cfn_record.get_or_insert(CfnRecord::default());
                cfn_record.cob = Some(value.to_owned());
                self.save_cfn_state();
                self.set_state(State::CfnRecord);
            }
            // `cfi` and `cfl` are the same, they are just written differently because of historical
            // reasons
            Some(("cfi" | "cfl", value)) => {
                let cfn_record = self.cfn_record.get_or_insert(CfnRecord::default());
                cfn_record.cfi = Some(value.to_owned());
                self.save_cfn_state();
                self.set_state(State::CfnRecord);
            }
            Some(("cfn", value)) => {
                let cfn_record = self.cfn_record.get_or_insert(CfnRecord::default());
                cfn_record.cfn = value.to_owned();
                self.save_cfn_state();
                self.set_state(State::CfnRecord);
            }
            Some(("calls", value)) => {
                let cfn_record = self.cfn_record.get_or_insert(CfnRecord::default());
                for (index, count) in value
                    .split_ascii_whitespace()
                    .map(|s| s.parse::<u64>().unwrap())
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
            Some(_) => self.handle_unknown_state(line, &split),
            None => self.handle_cost_line_state(line),
        }
    }

    // Doesn't set a state by itself so the next handled state is the state before ending up here
    fn handle_unknown_state(&mut self, line: &str, split: &Split) {
        if split.is_some() {
            trace!("Found unknown specification: {}. Skipping it ...", line);
        } else {
            self.handle_cost_line_state(line);
        }
    }

    fn handle_cost_line_state(&mut self, line: &str) {
        // We check if it is a line starting with a digit. If not, it is a misinterpretation of the
        // callgrind format so we panic here.
        assert!(
            line.starts_with(|c: char| c.is_ascii_digit()),
            "Costline must start with a digit"
        );

        // From the documentation of the callgrind format:
        // > If a cost line specifies less event counts than given in the "events" line, the
        // > rest is assumed to be zero.
        // TODO: WRAP COUNTS INTO STRUCT
        let mut costs: [u64; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
        for (index, counter) in line
                        .split_ascii_whitespace()
                        // skip the first number which is just the line number or instr number or
                          // in case of `instr line` skip 2
                        .skip(if self.positions_mode == PositionsMode::InstrLine { 2 } else { 1 })
                        .map(|s| s.parse::<u64>().expect("Encountered non ascii digit"))
                        // we're only interested in the counters for instructions and the cache
                        .take(9)
                        .enumerate()
        {
            costs[index] = counter;
        }

        // A cfn record takes precedence over a inline record (=fe/fi)
        if let Some(mut cfn_record) = self.cfn_record.take() {
            assert!(
                !cfn_record.cfn.is_empty(),
                "A cfn record must have an cfn entry"
            );

            cfn_record.costs = costs;
            let record = self.record.as_mut().unwrap();
            for (index, counter) in cfn_record.costs.iter().enumerate() {
                record.inclusive_costs[index] += counter;
            }

            cfn_record.file = match cfn_record.cfi.as_deref() {
                None | Some("???") => match cfn_record.cob.as_deref() {
                    None | Some("???") => self.target.as_ref().map(|(_, v)| v.clone()),
                    Some(value) => Some(value.to_owned()),
                },
                Some(value) => Some(value.to_owned()),
            };

            record.cfns.push(cfn_record);

            // A cfn record has exactly 1 cost line, so we can restore the state from before the cfn
            // state
            self.restore_cfn_state();

            // An inline record can have multiple cost lines so we cannot end an `InlineRecord`
            // here. Only another inline record can end an inlinerecord.
        } else if let Some(inline_record) = self.inline_record.as_mut() {
            let record = self.record.as_mut().unwrap();
            for (index, counter) in costs.iter().enumerate() {
                inline_record.costs[index] += counter;
                record.inclusive_costs[index] += counter;
            }
            self.set_state(State::InlineRecord);
            // Much like inline records, a Record can have mulitple cost lines.
        } else {
            let record = self.record.as_mut().unwrap();
            for (index, counter) in costs.iter().enumerate() {
                record.inclusive_costs[index] += counter;
                record.self_costs[index] += counter;
            }

            self.set_state(State::Record);
        }
    }
}

impl Default for LinesParser {
    fn default() -> Self {
        Self {
            positions_mode: PositionsMode::Line,
            record: Option::default(),
            cfn_record: Option::default(),
            inline_record: Option::default(),
            current_state: State::Header,
            old_state: Option::default(),
            target: Option::default(),
        }
    }
}

impl CallgrindParser for HashMapParser {
    fn parse<T>(&mut self, file: T) -> Result<()>
    where
        T: AsRef<super::CallgrindOutput>,
        Self: std::marker::Sized,
    {
        let file = File::open(&file.as_ref().file).unwrap();
        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap);
        if !iter
            .by_ref()
            .find(|l| !l.trim().is_empty())
            .expect("Non-empty file")
            .contains("callgrind format")
        {
            warn!("Missing file format specifier. Assuming callgrind format.");
        };

        LinesParser::default().parse(self, iter);
        Ok(())
    }
}
