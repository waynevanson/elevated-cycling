use anyhow::Result;
use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use std::process::Command;

#[test]
fn parses_simply() -> Result<()> {
    let mut command = Command::cargo_bin("elevated-cycling")?;

    command.assert().try_success()?;

    Ok(())
}
