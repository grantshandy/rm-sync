#![allow(unused_variables)]

use std::path;

use axum::{
    extract::{self, Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use webdav::methods as dav_methods;

use crate::rm::Filesystem;

pub async fn dav(
    method: Method,
    path: Option<extract::Path<path::PathBuf>>,
    State(fs): State<Filesystem>,
    req: Request,
) -> Response {
    let path = match path {
        Some(extract::Path(path)) => path,
        None => path::Path::new("/").into(),
    };

    match method {
        Method::GET => dav_get(req, path, fs).await,
        Method::PUT => dav_put(req, path, fs).await,
        Method::DELETE => dav_delete(req, path, fs).await,
        Method::OPTIONS => dav_options(req, path, fs).await,
        _ => {
            // this is a really bad way to do it but the only way unfortunately as dav_methods::* is a Lazy and not a constant.
            if method == dav_methods::COPY.as_ref() {
                dav_copy(req, path, fs).await
            } else if method == dav_methods::MOVE.as_ref() {
                dav_move(req, path, fs).await
            } else if method == dav_methods::MKCOL.as_ref() {
                dav_mkcol(req, path, fs).await
            } else if method == dav_methods::LOCK.as_ref() {
                dav_lock(req, path, fs).await
            } else if method == dav_methods::UNLOCK.as_ref() {
                dav_unlock(req, path, fs).await
            } else if method == dav_methods::PROPFIND.as_ref() {
                dav_propfind(req, path, fs).await
            } else if method == dav_methods::PROPPATCH.as_ref() {
                dav_proppatch(req, path, fs).await
            } else {
                StatusCode::METHOD_NOT_ALLOWED.into_response()
            }
        }
    }
}

async fn dav_get(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    match path.is_file() {
        true => "files not supported yet".into_response(),
        false => format!("{:#?}", fs.list(path)).into_response(),
    }
}

async fn dav_put(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    todo!()
}

async fn dav_delete(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    todo!()
}

async fn dav_options(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    todo!()
}

async fn dav_proppatch(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}

async fn dav_propfind(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}

async fn dav_unlock(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}

async fn dav_lock(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}

async fn dav_mkcol(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}

async fn dav_move(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}

async fn dav_copy(
    req: axum::http::Request<axum::body::Body>,
    path: path::PathBuf,
    fs: Filesystem,
) -> Response {
    todo!()
}
