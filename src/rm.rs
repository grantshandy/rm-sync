use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use dashmap::{DashMap, DashSet};
use futures::{stream, StreamExt};
use notify::{Event, RecursiveMode, Watcher};
use uuid::Uuid;

mod disk;

/// Time between file re-polls. Files are only read when updated, but batch updated when changed every POLL_DURATION
const POLL_DURATION: Duration = Duration::from_secs(2);

const PINNED_DIRECTORY: &str = "Favorites";
const TRASH_DIRECTORY: &str = "Trash";

/// A thread-safe representation of the reMarkable filesystem
#[derive(Debug, Clone, Default)]
pub struct Filesystem {
    /// The base path to the document filesystem.
    /// On the reMarkable device, this is '/home/root/.local/share/remarkable/xochitl/'.
    base: PathBuf,

    elements: DashMap<Uuid, Element>,
}

impl Filesystem {
    /// Construct a filesystem from its base path, indexing before returning.
    pub async fn from_path(path: impl Into<PathBuf>) -> Self {
        let me = Self {
            base: path.into(),
            ..Default::default()
        };

        tracing::info!("constructing Filesystem at {:?}", me.base);

        me.index().await;

        me
    }

    pub fn list(&self, path: impl Into<PathBuf>) -> Vec<Element> {
        let path = path.into();

        // can't list children of files
        if path.is_file() {
            tracing::warn!("list called on a file");
            return Vec::new();
        }

        // if path is root, simply return all files pointing to the root
        if path.parent().is_none() {
            return self
                .elements
                .iter()
                .filter(|e| e.value().parent == Parent::Root)
                .map(|v| v.value().clone())
                .collect();
        }

        // if path is trash, return all files in the trash
        if path == Path::new(TRASH_DIRECTORY) {
            return self
                .elements
                .iter()
                .filter(|e| e.parent == Parent::Trash)
                .map(|v| v.value().clone())
                .collect();
        }

        // if path is custom pinned path, return that
        if path == Path::new(PINNED_DIRECTORY) {
            return self
                .elements
                .iter()
                .filter(|e| e.pinned)
                .map(|e| e.value().clone())
                .collect();
        }

        let Some((uuid, _)) = self.uuid_from_path(&path) else {
            tracing::error!("no uuid found for directory {path:?}");
            return Vec::new();
        };

        return self
            .elements
            .iter()
            .filter(|e| e.value().parent == Parent::Directory(uuid))
            .map(|e| e.value().clone())
            .collect();
    }

    fn uuid_from_path(&self, path: &PathBuf) -> Option<(Uuid, Element)> {
        let mut components = path.components();

        let name = match components.next_back()? {
            Component::Normal(os) => os.to_string_lossy(),
            _ => return None,
        };

        let mut candidates: Vec<(Uuid, Element)> = self
            .elements
            .iter()
            .filter(|e| e.name == name)
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect();

        // if there are no elements with the same name, it must not exist
        if candidates.is_empty() {
            return None;
        }

        // if the name is unique, return that
        if candidates.len() == 1 {
            return candidates.pop();
        }

        candidates
            .iter()
            .find(|(_, elem)| self.path_matches(&path, &elem))
            .cloned()
    }

    /// A method for verifying that an element exists at a given path
    fn path_matches(&self, path: &Path, elem: &Element) -> bool {
        match (path.parent(), elem.parent) {
            // elem parent's UUID exists in self.elements & is a directory that ends with the current path, AND is itself valid
            (Some(parent), Parent::Directory(uuid)) => self
                .elements
                .get(&uuid)
                .map(|v| v.value().clone())
                .filter(|v| v.is_dir() && parent.ends_with(&v.name))
                .is_some_and(|v| self.path_matches(parent, &v)),

            // Parent::Trash <=> "/Trash"
            (Some(t), Parent::Trash) if t == Path::new(TRASH_DIRECTORY) => true,

            // Parent::Root <=> "/" (no parent)
            (None, Parent::Root) => true,

            // if all else, false
            _ => true,
        }
    }

    pub async fn index(&self) {
        tracing::info!("indexing");

        let dir = match self.base.read_dir() {
            Ok(dir) => dir,
            Err(err) => {
                tracing::error!("failed to read dir {:?}: {}", self.base, err);
                return;
            }
        };

        self.elements.clear();

        stream::iter(dir)
            // filter by Ok entries' paths
            .filter_map(|res| async move {
                match res {
                    Ok(entry) => Some(entry.path()),
                    Err(err) => {
                        tracing::warn!("couldn't read entry: {err}");
                        None
                    }
                }
            })
            // only valid .metadata paths
            .filter_map(|path| async move { disk::Metadata::validate_path(&path) })
            // read from disk and insert into the elements
            .for_each_concurrent(None, |uuid| async move {
                match disk::read(&self.base, &uuid).await {
                    Ok(element) => {
                        self.elements.insert(uuid, element);
                    }
                    Err(err) => tracing::error!("error reading {uuid} from disk: {err}"),
                }
            })
            .await;

        tracing::debug!("finished indexing");
    }

    /// Infinitely poll the base directory for changes and update Elements when modified and deleted.
    pub async fn auto_reindex(&self) {
        // insert into a set in order to reduce duplicate events (& therefore file reads) within POLL_DURATION
        let to_update: Arc<DashSet<Uuid>> = Arc::new(DashSet::new());
        let to_delete: Arc<DashSet<Uuid>> = Arc::new(DashSet::new());

        let watch_update = to_update.clone();
        let watch_delete = to_delete.clone();

        let watch_handler = move |res: Result<Event, notify::Error>| {
            let event = match res {
                Ok(e) => e,
                Err(err) => {
                    tracing::warn!("failed to watch file: {err:?}");
                    return;
                }
            };

            // not important for updating
            if event.kind.is_access() {
                return;
            }

            // filter changed paths by valid .metadata and .content files then insert them into the queue
            event
                .paths
                .iter()
                .filter_map(|path| {
                    disk::Metadata::validate_path(path).or(disk::Content::validate_path(path))
                })
                .for_each(|uuid| {
                    if event.kind.is_remove() {
                        watch_delete.insert(uuid);
                    } else {
                        watch_update.insert(uuid);
                    }
                });
        };

        let mut watcher = match notify::recommended_watcher(watch_handler) {
            Ok(w) => w,
            Err(err) => {
                tracing::error!("error building watcher: {err:?}");
                return;
            }
        };

        // watch our base directory
        if let Err(err) = watcher.watch(&self.base, RecursiveMode::Recursive) {
            tracing::error!("error watching root directory: {err}");
            return;
        }

        loop {
            tokio::time::sleep(POLL_DURATION).await;

            // remove all elements who were deleted
            to_delete.iter().for_each(|uuid| {
                let uuid = uuid.key();

                tracing::debug!("removing {uuid}");
                self.elements.remove(uuid);
            });
            to_delete.clear();

            // asynchronously update modified elements from disk
            stream::iter(to_update.iter())
                .for_each_concurrent(None, |uuid| async move {
                    let uuid = uuid.key();

                    match disk::read(&self.base, uuid).await {
                        Ok(element) => {
                            self.elements.insert(*uuid, element);
                        }
                        Err(err) => {
                            tracing::error!("failed to update {uuid} from disk: {err}");
                        }
                    }
                    tracing::debug!("updated {uuid} from disk");
                })
                .await;

            to_update.clear();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Document {
    format: Format,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    parent: Parent,
    pinned: bool,
    kind: ElementKind,
}

impl Element {
    pub fn is_doc(&self) -> bool {
        match self.kind {
            ElementKind::Document(_) => true,
            ElementKind::Directory => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self.kind {
            ElementKind::Document(_) => false,
            ElementKind::Directory => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ElementKind {
    Document(Document),
    Directory,
}

#[derive(serde::Deserialize, Eq, PartialEq, Debug, Copy, Clone)]
enum Format {
    #[serde(rename = "notebook")]
    Notebook,
    #[serde(rename = "pdf")]
    Pdf,
    #[serde(rename = "epub")]
    Epub,
}

#[derive(serde::Deserialize, Eq, PartialEq, Debug, Copy, Clone)]
enum Parent {
    #[serde(rename = "")]
    Root,
    #[serde(rename = "trash")]
    Trash,
    #[serde(untagged)]
    Directory(Uuid),
}
