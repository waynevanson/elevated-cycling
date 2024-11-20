use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{fs::File, io::Write, ops::Range, path::PathBuf};

use crate::AlphaPathSegment;

#[derive(Debug)]
pub struct Encoder {
    writer: File,
    rng: ThreadRng,
    path: PathBuf,
    range: Range<u64>,

    suffix: AlphaPathSegment,
    remaining: u64,
}

impl Encoder {
    pub fn try_new(path: PathBuf, range: Range<u64>, factor: usize) -> std::io::Result<Self> {
        let suffix: String = vec!['a'; factor].into_iter().collect();
        let full = path.join(&suffix);
        let suffix = AlphaPathSegment(suffix);
        std::fs::create_dir_all(&path)?;
        let writer = File::create(&full)?;
        let mut rng = thread_rng();
        let remaining = rng.gen_range(range.clone());

        Ok(Self {
            rng,
            path,
            writer,
            range,
            suffix,
            remaining,
        })
    }
}

impl Write for Encoder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let end = buf.len().min(self.remaining.try_into().unwrap());
        let chunk = &buf[0..end];

        let written = self.writer.write(chunk)? as u64;

        if written < self.remaining {
            // Still more to write
            self.remaining -= written;
        } else {
            // written all we can to this writer,
            // get ready for the next
            self.writer.flush()?;

            self.suffix.increment_mut();

            let filepath = self.path.join(&self.suffix);

            self.writer = File::create(filepath)?;
            self.remaining = self.rng.gen_range(self.range.clone());
        }

        Ok(written.try_into().unwrap())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
