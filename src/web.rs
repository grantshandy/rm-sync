use std::{path::PathBuf, str::FromStr, sync::Arc};

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    routing, Router,
};
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::remarkable::Remarkable;

pub fn router() -> Router<Arc<Remarkable>> {
    Router::new()
        .route("/", routing::get(root))
        .route("/explorer", routing::get(explorer))
        .fallback(routing::get(fallback))
}

async fn root(state: State<Arc<Remarkable>>) -> Response {
    page(
        "rm-cloudsync",
        html! {
            h1 { "rm-cloudsync" }
            (explorer(Query(ExplorerQuery { path: PathBuf::from_str("/Trash").unwrap() }), state).await)
        },
    ).into_response()
}

#[derive(serde::Deserialize, Default)]
struct ExplorerQuery {
    path: PathBuf,
}

async fn explorer(Query(query): Query<ExplorerQuery>, State(fs): State<Arc<Remarkable>>) -> Markup {
    // let elems = match fs.list(&query.path).await {
    //     Ok(elems) => elems,
    //     Err(err) => {
    //         return html! {
    //             "Error: " (format!("{err:#?}"))
    //         };
    //     }
    // };

    let elems = fs.pinned().await;

    html! {
        #explorer {
            p { "path: " (format!("{:?}", query.path)) }
            (format!("{elems:?}"))
        }
    }
}

async fn fallback() -> Markup {
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
                script { (PreEscaped(include_str!(env!("HTMX")))) }
                title { (title.as_ref()) }
            }
            body { (body) }
        }
    }
}
