//! Utilities for reading from the reMarkable operating system.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use color_eyre::eyre;
use serde_json::Value;
use tokio::fs;
use uuid::Uuid;

use super::{Document, Element, ElementKind, Format, Parent};

pub const METADATA_EXTENSION: &str = "metadata";
pub const CONTENT_EXTENSION: &str = "content";

pub async fn read(base: &PathBuf, uuid: &Uuid) -> eyre::Result<Element> {
    // read metadata
    let meta = Metadata::from_disk(base, uuid).await?;

    let kind: ElementKind = match meta.kind {
        ElementType::Document => {
            ElementKind::Document(Content::from_disk(base, &uuid).await?.into())
        }
        ElementType::Directory => ElementKind::Directory,
    };

    Ok(Element {
        name: meta.name,
        parent: meta.parent,
        pinned: meta.pinned,
        kind,
    })
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
enum ElementType {
    #[serde(rename = "DocumentType")]
    Document,
    #[serde(rename = "CollectionType")]
    Directory,
}

/// Representation of \<BASE\>/\<UUID\>.metadata
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Metadata {
    parent: Parent,
    pinned: bool,
    #[serde(rename = "type")]
    kind: ElementType,
    #[serde(rename = "visibleName")]
    name: String,
}

impl Metadata {
    pub async fn from_disk(base: &Path, uuid: &Uuid) -> eyre::Result<Self> {
        let mut path = base.join(uuid.to_string());
        path.set_extension(METADATA_EXTENSION);

        if !path.exists() {
            return Err(eyre::eyre!("{path:?} doesn't exist"));
        }

        Ok(serde_json::from_slice(&fs::read(path).await?)?)
    }

    pub fn validate_path(path: &Path) -> Option<Uuid> {
        if path.extension() == Some(METADATA_EXTENSION.as_ref()) {
            let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();

            Uuid::from_str(&file_stem).ok()
        } else {
            None
        }
    }
}

/// Representation of \<BASE\>/\<UUID\>.content
#[derive(serde::Deserialize, serde::Serialize, Eq, PartialEq, Debug, Copy, Clone)]
pub struct Content {
    #[serde(rename = "fileType")]
    format: Format,
}

impl Content {
    pub async fn from_disk(base: &Path, uuid: &Uuid) -> eyre::Result<Self> {
        let mut path = base.join(uuid.to_string());
        path.set_extension(CONTENT_EXTENSION);

        if !path.exists() {
            return Err(eyre::eyre!("{path:?} doesn't exist"));
        }

        Ok(serde_json::from_slice(&fs::read(path).await?)?)
    }

    pub fn validate_path(path: &Path) -> Option<Uuid> {
        if path.extension() == Some(CONTENT_EXTENSION.as_ref()) {
            let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();

            Uuid::from_str(&file_stem).ok()
        } else {
            None
        }
    }
}

impl Into<Document> for Content {
    fn into(self) -> Document {
        Document {
            format: self.format,
        }
    }
}

pub async fn change_parent(base: impl AsRef<Path>, uuid: &Uuid, parent: Parent) -> eyre::Result<()> {
    let mut path = base.as_ref().join(uuid.to_string());
    path.set_extension(METADATA_EXTENSION);

    let mut disk_value: Value = serde_json::from_slice(&fs::read(&path).await?)?;

    let parent = serde_json::to_value(parent)?;
    if let Some(disk_parent) = disk_value.get_mut("parent") {
        *disk_parent = parent;
    }

    fs::write(path, serde_json::to_string_pretty(&disk_value)?).await?;

    Ok(())
}
