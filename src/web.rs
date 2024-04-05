use std::{path::PathBuf, sync::Arc};

use axum::extract::{Query, State};
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::rm::Filesystem;

#[derive(serde::Deserialize)]
pub struct PageQuery {
    dir: Option<PathBuf>,
}

pub async fn root(Query(query): Query<PageQuery>, State(fs): State<Arc<Filesystem>>) -> Markup {
    let query_dir = query.dir.unwrap_or_default();
    let elems = fs.list(&query_dir);

    page(
        "rm-cloudsync",
        html! {
            h1 { "rm-cloudsync" }
            h3 { "Directories" }
            ul {
                @if let Some(parent) = query_dir.parent() {
                    li {
                        a href=(format!("/?dir={}", parent.to_string_lossy())) { "↰ Back" }
                    }
                }
                @for dir in elems.iter().filter(|e| e.is_dir()) {
                    li {
                        a href=(format!("/?dir={}/{}", query_dir.to_string_lossy(), dir.name)) { (dir.name) }
                    }
                }
            }
            h3 { "Documents" }
            ul {
                @for doc in elems.iter().filter(|e| e.is_doc()) {
                    li { (doc.name) }
                }
            }
        },
    )
}

pub async fn fallback() -> Markup {
    page(
        "Page not Found",
        html! {
            h1 { "404: Page not Found" }
            p { a href="/" {("Return to home")} }
        },
    )
}

fn page(title: impl AsRef<str>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                // script { (PreEscaped(include_str!(env!("HTMX")))) }
                title { (title.as_ref()) }
            }
            body { (body) }
        }
    }
}
