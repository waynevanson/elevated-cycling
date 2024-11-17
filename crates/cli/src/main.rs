use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::LevelFilter;
use elevated_cycling_cli::{ParsedArgs, RawArgs};

fn main() -> Result<()> {
    let args = try_get_args()?;

    Ok(())
}

fn setup_logger(level: LevelFilter) -> Result<()> {
    env_logger::Builder::new().filter_level(level).try_init()?;
    Ok(())
}

fn try_get_args() -> Result<ParsedArgs> {
    let raw_args = RawArgs::try_parse()?;

    setup_logger(raw_args.verbose.log_level_filter())?;

    let args = ParsedArgs::from(raw_args);

    Ok(args)
}
