/// Obfustcat is the love child of `size` with an prng.
///
/// Target state:
/// encoding: tar | xz | split m..M
/// decoding: cat parts_*2 | xz | tar
use anyhow::Result;
use clap::Parser;
use rand::Rng;
use std::{
    fs::{read_dir, File},
    io::{BufReader, Read},
    path::PathBuf,
    process,
};

#[derive(Debug, Parser)]
struct Encode {
    file: Option<PathBuf>,

    #[arg(short, long)]
    prefix: PathBuf,

    #[arg(short = 'm', long, default_value_t = 10 * 1_000_000)]
    min: usize,

    #[arg(short = 'M', long, default_value_t = 1_000 * 1_000_000)]
    max: usize,
}

// todo: data that comes reader determines at `m` use to get next random amount
// so it's deterministic.
impl Encode {
    fn encode(self) -> Result<()> {
        let mut reader: BufReader<Box<dyn Read>> = if let Some(pathbuf) = self.file {
            BufReader::new(Box::new(File::open(pathbuf)?))
        } else {
            BufReader::new(Box::new(std::io::stdin().lock()))
        };

        let mut rng = rand::thread_rng();
        let range = self.min..self.max;
        let mut part_count = 0;

        loop {
            // read a chunk
            let chunk_size = rng.gen_range(range.clone());
            let mut chunk = vec![0; chunk_size];
            let read = reader.read(&mut chunk)?;
            if read == 0 {
                break;
            };
            chunk.truncate(read);

            part_count += 1;
            // move to file
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Encode::parse();

    Ok(())
}
