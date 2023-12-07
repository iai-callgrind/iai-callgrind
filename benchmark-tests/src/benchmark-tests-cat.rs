use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

const FILE_WITH_CONTENT: &str = "fixtures/file_with_content.txt";
const FILE_WITHOUT_CONTENT: &str = "fixtures/file_without_content.txt";
const FILE_WITH_CONTENT_EXPECTED: &[u8] = b"one\ntwo\n";

fn main() -> Result<(), std::io::Error> {
    let file = std::env::args()
        .nth(1)
        .expect("Argument with filename to print to stdout");
    let actual = cat(&PathBuf::from(&file))?;
    match file.as_str() {
        FILE_WITH_CONTENT => {
            assert_eq!(actual, FILE_WITH_CONTENT_EXPECTED);
        }
        FILE_WITHOUT_CONTENT => {
            assert_eq!(actual, Vec::<u8>::default().as_slice());
        }
        _ => {}
    }

    std::io::stdout().write_all(&actual)?;
    std::io::stdout().flush()
}

#[inline(never)]
fn cat(file: &Path) -> Result<Vec<u8>, std::io::Error> {
    let mut reader = BufReader::new(File::open(file)?);
    let mut buffer = vec![];
    reader.read_to_end(&mut buffer).map(|_| buffer)
}
