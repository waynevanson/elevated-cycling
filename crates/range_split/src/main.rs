use anyhow::Result;
use bytesize::ByteSize;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use range_split::{try_from_str, FileSplitEncoder};
use rangetools::BoundedRange;
use std::{fs::File, io::Read, ops::Range, path::PathBuf};

#[derive(Debug, Parser)]
struct Encode {
    /// the path prefixed to the start of the generated file.
    /// Can be a directory or a path.
    #[arg(short, long, default_value = r#""""#)]
    prefix: Option<PathBuf>,

    /// How many characters file names will be.
    /// `aaaa` is the default start when set to `4`.
    ///
    /// When this overflows, it will be `zzzz[a-z]` then `zzzzz[a-z]`.
    #[arg(short, long, default_value_t = 4)]
    factor: usize,

    #[command(flatten)]
    verbosity: Verbosity,

    /// The range of file sizes that can be generated.
    /// The last generated file has no lower bound.
    ///
    /// [n,N] (n,N) [n,N) (n,N]
    #[arg(value_parser = try_from_str::<ByteSize>)]
    range: BoundedRange<ByteSize>,

    /// The file to read contents from.
    /// If omitted, defaults to standard input.
    file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Encode::parse();

    env_logger::builder()
        .filter_level(args.verbosity.log_level_filter())
        .try_init()?;

    let mut reader = create_source_reader(args.file)?;

    let range = from_bounded_bytesize_to_u64(args.range);
    let mut writer = FileSplitEncoder::new(
        args.prefix.unwrap_or_else(|| PathBuf::from("")),
        range,
        args.factor,
    );

    std::io::copy(&mut reader, &mut writer)?;

    Ok(())
}

fn create_source_reader(file: Option<PathBuf>) -> Result<impl Read> {
    let reader: Box<dyn Read> = if let Some(path) = file {
        let reader = File::open(path)?;
        Box::new(reader)
    } else {
        let reader = std::io::stdin().lock();
        Box::new(reader)
    };

    Ok(reader)
}

fn from_bounded_bytesize_to_u64(range: BoundedRange<ByteSize>) -> Range<u64> {
    let start = range.start.map(|bytesize| bytesize.as_u64());
    let end = range.end.map(|bytesize| bytesize.as_u64());
    let range = Range::from(BoundedRange { start, end });
    range
}
