use std::{fs, io, path::PathBuf};

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
    root_dir: PathBuf,
    documents: Vec<Document>,
}

impl RmFS {
    pub async fn from_path(dir: impl Into<PathBuf>) -> io::Result<Self> {
        let root_dir: PathBuf = dir.into();
        let mut documents = Vec::new();

        tracing::info!("parsing document directory {root_dir:?}");

        // build metadata
        for entry in root_dir.read_dir()?.filter_map(Result::ok).filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|path| path.to_ascii_lowercase() == "metadata")
        }) {
            let metadata: Metadata = match serde_json::from_slice(&fs::read(entry.path())?) {
                Ok(metadata) => metadata,
                Err(err) => {
                    tracing::error!("{err:?}");
                    continue;
                }
            };

            documents.push(Document { metadata });
        }

        Ok(Self {
            root_dir,
            documents,
        })
    }
}

#[derive(Debug, Clone)]
struct Document {
    metadata: Metadata,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Metadata {
    #[serde(rename = "type")]
    element_type: Type,
    #[serde(rename = "visibleName")]
    title: String,

    parent: String,
}

#[derive(serde::Deserialize, Debug, Copy, Clone)]
enum Type {
    #[serde(rename = "DocumentType")]
    Document,
    #[serde(rename = "CollectionType")]
    Collection,
}
