use std::{
    fs::File,
    io::{BufReader, Result},
    path::Path,
};

use osmpbf::ElementReader;

pub trait ElementReaderExt: Sized {
    fn with_capacity(capacity: usize, path: impl AsRef<Path>) -> Result<Self>;
}

impl ElementReaderExt for ElementReader<BufReader<File>> {
    fn with_capacity(capacity: usize, path: impl AsRef<Path>) -> Result<Self> {
        let osm = BufReader::with_capacity(capacity, File::open(path)?);
        let pbf = ElementReader::new(osm);
        Ok(pbf)
    }
}
