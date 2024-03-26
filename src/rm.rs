use std::{
    io,
    path::PathBuf,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Documents {
    root_dir: PathBuf,
}

impl Documents {
    pub async fn from_path(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let root_dir = dir.into();

        tracing::info!("parsing document directory {root_dir:?}");

        Ok(Self {
            root_dir,
        })
    }
}
