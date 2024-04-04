#![allow(unused_variables)]

use std::path;

use crate::rm::Filesystem;
use axum::{
    extract::{self, Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use headers_core::HeaderValue;
use webdav::methods::{COPY, LOCK, MKCOL, MOVE, PROPFIND, PROPPATCH, UNLOCK};

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

    let mut resp = match method {
        Method::GET => dav_get(req, path, fs).await,
        Method::PUT => dav_put(req, path, fs).await,
        Method::DELETE => dav_delete(req, path, fs).await,
        Method::OPTIONS => dav_options(req, path, fs).await,
        _ if method == COPY.as_ref() => dav_copy(req, path, fs).await,
        _ if method == MOVE.as_ref() => dav_move(req, path, fs).await,
        _ if method == MKCOL.as_ref() => dav_mkcol(req, path, fs).await,
        _ if method == LOCK.as_ref() => dav_lock(req, path, fs).await,
        _ if method == UNLOCK.as_ref() => dav_unlock(req, path, fs).await,
        _ if method == PROPFIND.as_ref() => dav_propfind(req, path, fs).await,
        _ if method == PROPPATCH.as_ref() => dav_proppatch(req, path, fs).await,
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    };

    resp.headers_mut()
        .append("dav", HeaderValue::from_static("1"));

    resp
}

async fn dav_get(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    format!("{:#?}", fs.list(path)).into_response()
}

async fn dav_put(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_delete(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_options(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_proppatch(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_propfind(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_lock(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    StatusCode::BAD_REQUEST.into_response()
}

async fn dav_unlock(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    StatusCode::BAD_REQUEST.into_response()
}

async fn dav_mkcol(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_move(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}

async fn dav_copy(req: Request, path: path::PathBuf, fs: Filesystem) -> Response {
    ().into_response()
}
