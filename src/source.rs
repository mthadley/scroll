use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Error, ErrorKind, Read, Result};

/// Represents the input source for scroll.
pub enum Source {
    FromFile(File),
    FromStdin(io::Stdin),
}

impl Read for Source {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            Source::FromFile(f) => f.read(buf),
            Source::FromStdin(s) => s.read(buf),
        }
    }
}

impl From<File> for Source {
    fn from(file: File) -> Self {
        Source::FromFile(file)
    }
}

impl From<io::Stdin> for Source {
    fn from(stdin: io::Stdin) -> Self {
        Source::FromStdin(stdin)
    }
}

impl From<Source> for BufReader<Source> {
    fn from(source: Source) -> Self {
        BufReader::new(source)
    }
}

/// Attempts to read a file from the passed arguments, or defaults
/// to reading data from stdin.
pub fn get_source() -> Result<impl BufRead> {
    env::args()
        .nth(1)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Missing file"))
        .and_then(File::open)
        .map(Source::from)
        .or_else(|_| Ok(Source::from(io::stdin())))
        .map(BufReader::from)
}
