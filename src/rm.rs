use std::{
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use dashmap::DashMap;
use itertools::Itertools;
use uuid::Uuid;

/// Try to guess a good default document path based on the OS
pub fn default_doc_path() -> PathBuf {
    // try to detect armv7/linux/musl config for the remarkable itself
    #[cfg(all(target_arch = "arm", target_env = "musl", target_os = "linux"))]
    return "/home/root/.local/share/remarkable/xochitl/".into();

    // else use the sample files for development
    "./samples/v6/".into()
}

#[derive(Debug, Clone, Default)]
pub struct Filesystem {
    path: PathBuf,
    elements: DashMap<Uuid, Element>,
}

impl Filesystem {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let mut me = Self {
            path: path.into(),
            ..Default::default()
        };

        me.reindex();

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
        if path == Path::new("/Trash") {
            return self
                .elements
                .iter()
                .filter(|e| e.parent == Parent::Trash)
                .map(|v| v.value().clone())
                .collect();
        }

        // if path is custom pinned path, return that
        if path == Path::new("/Pinned") {
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

        unimplemented!("duplicate element names")
    }

    fn reindex(&mut self) {
        tracing::info!("indexing {:?}", self.path);

        let dir = match self.path.read_dir() {
            Ok(dir) => dir,
            Err(err) => {
                tracing::error!("failed to read dir {:?}: {}", self.path, err);
                return;
            }
        };

        let uuids: Vec<Uuid> = dir
            .filter_map(Result::ok)
            .filter_map(|s| {
                Uuid::from_str(&s.path().file_stem().unwrap_or_default().to_string_lossy()).ok()
            })
            .dedup()
            .collect();

        self.elements.clear();

        // TODO: parallelize?
        for uuid in uuids {
            if let Some(element) = disk::read(&self.path, uuid) {
                self.elements.insert(uuid, element);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Document {
    format: Format,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    name: String,
    parent: Parent,
    pinned: bool,
    content: ElementKind,
}

impl Element {
    pub fn is_doc(&self) -> bool {
        match self.content {
            ElementKind::Document(_) => true,
            ElementKind::Directory => false,
        }
    }

    pub fn is_dir(&self) -> bool {
        match self.content {
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

mod disk {
    use std::{fs, path::PathBuf};

    use uuid::Uuid;

    use super::{Document, Element, ElementKind, Format, Parent};

    pub fn read(path: &PathBuf, uuid: Uuid) -> Option<Element> {
        let metadata_path = path.join(format!("{uuid}.metadata"));

        if !metadata_path.exists() {
            return None;
        }

        // read metadata
        let meta: Metadata =
            match fs::read(metadata_path).map(|b| serde_json::from_slice::<Metadata>(&b)) {
                Ok(Ok(meta)) => meta,
                Ok(Err(err)) => {
                    tracing::error!("failed to parse {uuid}.metadata json: {err}");
                    return None;
                }
                Err(err) => {
                    tracing::error!("failed to read {uuid}.metadata: {err}");
                    return None;
                }
            };

        match meta.kind {
            ElementType::Document => {
                let content_path = path.join(format!("{uuid}.content"));

                if !content_path.exists() {
                    tracing::error!(
                        "content file {content_path:?} doesn't exist, skipping document"
                    );
                    return None;
                }

                // read .content file
                let content: Content =
                    match fs::read(content_path).map(|b| serde_json::from_slice::<Content>(&b)) {
                        Ok(Ok(content)) => content,
                        Ok(Err(err)) => {
                            tracing::error!("failed to parse {uuid}.content json: {err}");
                            return None;
                        }
                        Err(err) => {
                            tracing::error!("failed to read {uuid}.content: {err}");
                            return None;
                        }
                    };

                Some(Element {
                    name: meta.name,
                    parent: meta.parent,
                    pinned: meta.pinned,
                    content: ElementKind::Document(Document {
                        format: content.format,
                    }),
                })
            }
            ElementType::Directory => Some(Element {
                name: meta.name,
                parent: meta.parent,
                pinned: meta.pinned,
                content: ElementKind::Directory,
            }),
        }
    }

    /// Representation of <UUID>.metadata
    #[derive(serde::Deserialize, Debug)]
    struct Metadata {
        #[serde(rename = "type")]
        kind: ElementType,
        #[serde(rename = "visibleName")]
        name: String,
        parent: Parent,
        pinned: bool,
    }

    #[derive(serde::Deserialize, PartialEq, Debug)]
    enum ElementType {
        #[serde(rename = "DocumentType")]
        Document,
        #[serde(rename = "CollectionType")]
        Directory,
    }

    /// Representation of <UUID>.content
    #[derive(serde::Deserialize, Eq, PartialEq, Debug, Copy, Clone)]
    struct Content {
        #[serde(rename = "fileType")]
        format: Format,
    }
}
