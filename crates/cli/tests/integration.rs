use anyhow::Result;
use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use std::{fs, process::Command};

#[test]
fn parses_simply() -> Result<()> {
    let mut command = Command::cargo_bin("elevated-cycling")?;

    command.assert().try_success()?;

    Ok(())
}

// before test
// assert that the thing is there

fn verify_bulk() -> Result<()> {
    if fs::exists("bulk/images/elevated-cycling/data/planet.osm.pbf")? {
        panic!("Expected the bulk dir to exist")
    };

    Ok(())
}

#[test]
fn exists() -> Result<()> {
    verify_bulk()?;
    Ok(())
}
