use crate::AlphaPathSegment;
use bytesize::ByteSize;
use log::debug;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::{
    fs::{self, File},
    io::Write,
    ops::Range,
    path::PathBuf,
};

/// A file split encoder that transforms a file input to a file output.
#[derive(Debug)]
pub struct FileSplitEncoder {
    /// Used to ensure the PRNG only outputs within the specified range.
    range: Range<u64>,

    /// The current suffix for the file we're writing to.
    prefix: PathBuf,

    /// The current suffix we're writing to.
    /// This gets incremented to the next value once a file has been written to.
    suffix: AlphaPathSegment,

    /// The PRNG for determining how big a file should be.
    /// This is only created when we start writing,
    /// so it can be seeded the buffer contents.
    rng: Option<SmallRng>,

    /// The target file to write to.
    /// When a new file is filled to the size we want,
    /// flush the current writer and replace it.
    writer: Option<File>,

    /// Amount of bytes that can be written to the file before
    /// it is at capacity.
    remaining: Option<u64>,
}

impl FileSplitEncoder {
    /// Creates a new encoder for writing files.
    pub fn new(prefix: PathBuf, range: Range<u64>, factor: usize) -> Self {
        let suffix = AlphaPathSegment::from_factor(factor);

        Self {
            prefix,
            range,
            rng: None,
            suffix,
            remaining: None,
            writer: None,
        }
    }

    fn path(&self) -> PathBuf {
        let mut path = self.prefix.clone();
        path.as_mut_os_string().push(&self.suffix.0);
        path
    }

    fn increment(&mut self) {
        self.suffix.increment_mut();
    }
}

impl Write for FileSplitEncoder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let None = self.rng {
            self.rng = Some(SmallRng::seed_from_u64(*buf.get(0).unwrap() as u64));
            fs::create_dir_all(self.path().parent().unwrap())?;
        };

        if let None = self.writer {
            self.writer = Some(File::create(self.path())?);
        }

        if let None = self.remaining {
            let remaining = self.rng.as_mut().unwrap().gen_range(self.range.clone());
            debug!("Size: {}, File: {:?}", ByteSize::b(remaining), self.path());
            self.remaining = Some(remaining);
        };

        let remaining = self.remaining.as_mut().unwrap();

        let max = buf.len().min(*remaining as usize);
        let chunk = &buf[0..max];

        let written = self.writer.as_mut().unwrap().write(chunk)?;
        let pushed = written as u64;

        if pushed < *remaining {
            *remaining -= pushed;
        } else {
            debug!("Chunk complete");
            self.remaining = None;
            self.writer = None;
            self.increment();
        }

        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush()?;
        };

        Ok(())
    }
}
