use std::fs::File;
use std::path::PathBuf;

use inferno::flamegraph::Options;
use log::warn;

use super::callgrind::CallgrindOutput;
use super::IaiCallgrindError;
use crate::error::Result;

pub struct Stack(String);

// pub struct Stacks(Vec<Stack>);
// impl Stacks {
//     fn is_empty(&self) -> bool {
//         todo!()
//     }
// }

pub struct Flamegraph {
    pub title: String,
    pub stacks: Vec<String>,
}

pub struct FlamegraphOutput(pub PathBuf);

impl FlamegraphOutput {
    pub fn create(output: &CallgrindOutput) -> Result<Self> {
        let path = output.with_extension("svg").path;
        if path.exists() {
            let old_svg = path.with_extension("svg.old");
            std::fs::copy(&path, &old_svg).map_err(|error| {
                IaiCallgrindError::Other(format!(
                    "Error copying flamegraph file '{}' -> '{}' : {error}",
                    &path.display(),
                    &old_svg.display(),
                ))
            })?;
        }

        Ok(Self(path))
    }

    pub fn create_file(&self) -> Result<File> {
        File::create(&self.0).map_err(|error| {
            IaiCallgrindError::Other(format!("Creating flamegraph file failed: {error}"))
        })
    }
}

impl Flamegraph {
    pub fn create(&self, dest: &FlamegraphOutput) -> Result<()> {
        if self.stacks.is_empty() {
            warn!("Unable to create a flamegraph: Callgrind didn't record any events");
            return Ok(());
        }

        let output_file = dest.create_file()?;
        let mut options = Options::default();
        // TODO: count_name must be adjusted to the user chosen EventType. Best to use Display of
        // this EventType
        options.count_name = "Instructions".to_owned();
        options.title = self.title.clone();

        inferno::flamegraph::from_lines(
            &mut options,
            self.stacks.iter().map(std::string::String::as_str),
            output_file,
        )
        .map_err(|error| {
            crate::error::IaiCallgrindError::Other(format!(
                "Creating flamegraph file failed: {error}"
            ))
        })
    }
}
