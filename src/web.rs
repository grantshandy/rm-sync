use axum::extract::State;
use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::rm::Filesystem;

pub async fn root(State(fs): State<Filesystem>) -> Markup {
    page(
        "rm-cloudsync",
        html! {
            h1 { "rm-cloudsync" }
            pre { (PreEscaped(format!("{fs:#?}"))) }
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
                script { (PreEscaped(include_str!(env!("HTMX")))) }
                title { (title.as_ref()) }
            }
            body { (body) }
        }
    }
}
