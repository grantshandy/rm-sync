use std::{collections::HashMap, ffi::OsStr, fs, io, path::PathBuf};

use uuid::Uuid;

/// Try to guess a good default document path based on the OS
pub fn default_doc_path() -> PathBuf {
    // try to detect armv7/linux/musl config for the remarkable itself
    #[cfg(all(target_arch = "arm", target_env = "musl", target_os = "linux"))]
    return "/home/root/.local/share/remarkable/xochitl/".into();

    // else use the sample files for development
    "./samples/v6/".into()
}

#[derive(Debug, Clone)]
pub struct RmFS {
    pub root_dir: PathBuf,
    pub documents: HashMap<PathBuf, Document>,
}

impl RmFS {
    pub async fn from_path(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let root_dir: PathBuf = dir.into();

        let mut myself = Self {
            root_dir,
            documents: HashMap::new(),
        };

        myself.reindex().await?;

        Ok(myself)
    }

    async fn reindex(&mut self) -> io::Result<()> {
        tracing::debug!("indexing document directory {:?}", self.root_dir);

        // build metadata entries
        let mut parsed_documents: HashMap<Uuid, Metadata> = HashMap::new();
        let mut parsed_directories: HashMap<Uuid, Metadata> = HashMap::new();

        for entry in self.root_dir.read_dir()?.filter_map(Result::ok) {
            let path = entry.path();

            // only analyze metadata files
            if path.extension() != Some(OsStr::new("metadata")) {
                continue;
            }

            let metadata: Metadata = match serde_json::from_slice(&fs::read(&path)?) {
                Ok(metadata) => metadata,
                Err(err) => {
                    tracing::error!("couldn't parse json for {path:?}: {err:?}");
                    continue;
                }
            };

            let uuid = match path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(Uuid::parse_str)
            {
                Some(Ok(uuid)) => uuid,
                Some(Err(err)) => {
                    tracing::error!("error parsing {path:?} path UUID: {err:?}");
                    continue;
                }
                None => {
                    tracing::error!("No file stem found for {path:?}");
                    continue;
                }
            };

            match metadata.element_type {
                Type::Document => {
                    parsed_documents.insert(uuid, metadata);
                }
                Type::Collection => {
                    parsed_directories.insert(uuid, metadata);
                }
            };
        }

        self.documents.clear();
        for (uuid, metadata) in parsed_documents {
            let mut path = PathBuf::new();
            write_path(&parsed_directories, metadata.parent, &mut path);
            path.push(&metadata.title);

            self.documents.insert(path, Document { uuid, metadata });
        }

        Ok(())
    }
}

fn write_path(dirs: &HashMap<Uuid, Metadata>, current: Parent, out: &mut PathBuf) {
    match current {
        Parent::Root => (),
        Parent::Trash => out.push("Trash"),
        Parent::Directory(d) => match dirs.get(&d) {
            Some(meta) => {
                write_path(dirs, meta.parent, out);
                out.push(&meta.title);
            }
            None => {
                tracing::warn!("couldn't find parent directory {d} for {current:?}");
            }
        },
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub uuid: Uuid,
    pub metadata: Metadata,
}

/// Representation of <UUID>.metadata
#[derive(serde::Deserialize, Debug, Clone)]
struct Metadata {
    #[serde(rename = "type")]
    element_type: Type,
    #[serde(rename = "visibleName")]
    title: String,

    parent: Parent,

    // pinned: bool,
}

#[derive(serde::Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
enum Type {
    /// A document
    #[serde(rename = "DocumentType")]
    Document,
    /// A folder
    #[serde(rename = "CollectionType")]
    Collection,
}

#[derive(serde::Deserialize, Debug, Copy, Clone)]
enum Parent {
    #[serde(rename = "")]
    Root,
    #[serde(rename = "trash")]
    Trash,
    #[serde(untagged)]
    Directory(Uuid),
}
