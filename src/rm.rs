use itertools::Itertools;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};
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
pub struct Filesystem {
    path: PathBuf,
    root: Directory,
}

impl Filesystem {
    pub async fn from_path(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        let root = Directory::from_dir(&path).await?;

        Ok(Filesystem { path, root })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    name: String,
    uuid: Uuid,
    pinned: bool,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Directory {
    name: String,
    documents: Vec<Document>,
    directories: Vec<Directory>,
}

impl Directory {
    /// Construct a root [`Directory`] from a given path.
    pub async fn from_dir(path: &PathBuf) -> io::Result<Self> {
        tracing::debug!("indexing document directory {path:?}");

        // build metadata entries
        let mut parsed_documents: HashMap<Uuid, (Document, Parent)> = HashMap::new();
        let mut parsed_directories: HashMap<Uuid, (String, Parent)> = HashMap::new();

        // TODO: Parallelize?
        for entry in path.read_dir()? {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(err) => {
                    tracing::warn!("couldn't read file entry: {err}");
                    continue;
                }
            };

            // only analyze metadata files
            if path.extension() != Some(OsStr::new("metadata")) {
                continue;
            }

            // parse filename into typed Uuid
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
                    tracing::error!("no file stem found for {path:?}");
                    continue;
                }
            };

            let path = path.parent().unwrap();

            match read_metadata(&path, uuid) {
                Some(DiskEntry::Directory { name, parent, uuid }) => {
                    parsed_directories.insert(uuid, (name, parent));
                }
                Some(DiskEntry::Document { doc, parent }) => {
                    parsed_documents.insert(doc.uuid, (doc, parent));
                }
                None => (),
            };
        }

        // populate filesystem
        let mut root = Directory::default();

        // resolve document paths
        for (_, (doc, parent)) in parsed_documents {
            root.insert(&write_path(&parsed_directories, parent), doc);
        }

        Ok(root)
    }

    pub fn insert(&mut self, path: &[&str], target: Document) {
        match path.is_empty() {
            true => {
                if !self.documents.contains(&target) {
                    self.documents.push(target);
                }
            }
            false => {
                if !self.directories.iter().any(|d| d.name == path[0]) {
                    self.directories.push(Directory {
                        name: path[0].to_string(),
                        ..Default::default()
                    });
                }

                let dir = self
                    .directories
                    .iter_mut()
                    .find(|d| d.name == path[0])
                    .expect("get inserted child path");

                dir.insert(&path[1..], target);
            }
        }
    }

    pub fn get_mut(&mut self, uuid: &Uuid) -> Option<&mut Document> {
        for doc in &mut self.documents {
            if &doc.uuid == uuid {
                return Some(doc);
            }
        }

        for dir in &mut self.directories {
            if let Some(doc) = dir.get_mut(uuid) {
                return Some(doc);
            }
        }

        None
    }

    pub fn pinned(&self) -> Vec<Document> {
        let mut current: Vec<Document> = self.documents.iter().filter(|d| d.pinned).cloned().collect();
        let mut res = self
            .directories
            .iter()
            .flat_map(|dir| dir.pinned())
            .collect::<Vec<_>>();
        res.append(&mut current);

        return res;
    }
}

/// Continuously update a [`Directory`] when updated.
pub async fn watch_fs(fs: Arc<RwLock<Filesystem>>) {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| match res {
            Ok(event) => match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    if let Err(err) = tx.send(event) {
                        tracing::error!("error sending watch event: {err}");
                    }
                }
                _ => (),
            },
            _ => (),
        },
        Config::default(),
    )
    .expect("construct watcher");

    watcher
        .watch(&fs.read().await.path, RecursiveMode::Recursive)
        .expect("watch files");

    while let Some(e) = rx.recv().await {
        let uuids: Vec<Uuid> = e
            .paths
            .iter()
            .filter_map(|p| p.file_stem())
            .filter_map(|os| os.to_str())
            .filter_map(|s| Uuid::from_str(s).ok())
            .dedup()
            .collect();

        if uuids.is_empty() {
            continue;
        }

        let uuid = uuids[0];

        match e.kind {
            EventKind::Create(_) => tracing::debug!("created {uuids:?}"),
            EventKind::Modify(_) => {
                // update filesystem metadata from disk
                if let Some(DiskEntry::Document { doc, .. }) =
                    read_metadata(&fs.read().await.path, uuid)
                {
                    tracing::info!("got metadata");
                    let mut fs = fs.write().await;
                    tracing::info!("got write lock");
                    *fs.root.get_mut(&uuid).expect("get modified uuid") = doc;
                    drop(fs);
                };
            }
            EventKind::Remove(_) => tracing::debug!("removed {uuids:?}"),
            _ => (),
        }

        tracing::info!("finished updating");
    }
}

/// Recursively generate the path for a document from its parent.
fn write_path(dirs: &HashMap<Uuid, (String, Parent)>, current: Parent) -> Vec<&str> {
    match current {
        Parent::Root => vec![],
        Parent::Trash => vec!["Trash"],
        Parent::Directory(d) => match dirs.get(&d) {
            Some((name, parent)) => {
                let mut back = write_path(dirs, *parent);
                back.push(name);
                back
            }
            None => {
                tracing::warn!("couldn't find parent directory {d} for {current:?}");
                vec![]
            }
        },
    }
}

fn read_metadata(path: &Path, uuid: Uuid) -> Option<DiskEntry> {
    let path = path.join(format!("{uuid}.metadata"));
    tracing::debug!("updating {path:?}");

    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(err) => {
            tracing::error!("error reading .metadata: {err}");
            return None;
        }
    };

    let metadata: Metadata = match serde_json::from_slice(&bytes) {
        Ok(metadata) => metadata,
        Err(err) => {
            tracing::error!("error parsing json for {path:?}: {err:?}");
            return None;
        }
    };

    match metadata.element_type {
        Type::Document => Some(DiskEntry::Document {
            doc: Document {
                name: metadata.title,
                uuid,
                pinned: metadata.pinned,
            },
            parent: metadata.parent,
        }),
        Type::Collection => Some(DiskEntry::Directory {
            name: metadata.title,
            parent: metadata.parent,
            uuid,
        }),
    }
}

enum DiskEntry {
    Directory {
        name: String,
        parent: Parent,
        uuid: Uuid,
    },
    Document {
        doc: Document,
        parent: Parent,
    },
}

/// Representation of <UUID>.metadata
#[derive(serde::Deserialize, Debug)]
pub struct Metadata {
    #[serde(rename = "type")]
    element_type: Type,
    #[serde(rename = "visibleName")]
    title: String,
    parent: Parent,
    pinned: bool,
}

#[derive(serde::Deserialize, PartialEq, Debug)]
pub enum Type {
    /// A document
    #[serde(rename = "DocumentType")]
    Document,
    /// A folder
    #[serde(rename = "CollectionType")]
    Collection,
}

#[derive(serde::Deserialize, Eq, PartialEq, Debug, Copy, Clone)]
pub enum Parent {
    #[serde(rename = "")]
    Root,
    #[serde(rename = "trash")]
    Trash,
    #[serde(untagged)]
    Directory(Uuid),
}
