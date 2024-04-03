# rm-webdav (Work in Progress)

An open-source [WebDAV](https://en.wikipedia.org/wiki/WebDAV) server for the [reMarkable](https://remarkable.com/) tablet which enables you to sync all your notes to nearly any cloud service.

## How it works
```
**********************************      ************      ********************************      ******************
*                                *      *          *      *                              *      *                *
*  cloud service of your choice  * <--> *  rclone  * <--> *  WebDAV compatibility layer  * <--> *  device files  *
*                                *      *          *      *                              *      *                *
**********************************      ************      ********************************      ******************
                                                                         ^
                                                                         |
                                                                         v
                                                             ***************************
                                                             *                         *
                                                             *  web app file explorer  *
                                                             *                         *
                                                             ***************************
```

`rm-webdav` runs a WebDAV server on the actual tablet that parses and serves the tablet's on-disk representation of your notes.
From there, the WebDAV endpoint is consumed by [`rclone`](https://rclone.org/), a tool which can sync the files with [nearly any cloud service](https://rclone.org/#providers).
It also powers an HTTP file explorer/configuration panel for conveniently accessing the device while on the same network.

## Goals (in no particular order)
 - Support the latest reMarkable software (>= v3) first.
 - Render notes with near-perfect accuracy.
 - No data-loss.
 - Be performant by utilizing caching/parallelization and reducing memory allocations/network calls/file reads as much as possible as to not impact user experience and battery life.
 - Simple user experience (shell script to install over USB)
 - Simple installation (two statically-linked executables and a systemd service)

## Progress
 - [X] Cross-compile a statically-linked Rust executable with nix.
 - [ ] Read reMarkable filesystem
   - [X] Parse `.metadata`
   - [ ] Parse `.content`
   - [ ] Fetch PDF/EPUB documents
   - [ ] Fetch thumbnails
   - [ ] Automatically update representation when files are changed.
 - [ ] Methods for modifying files
   - [ ] Renaming notes/directories
   - [ ] Deleting documents/directories
   - [ ] Creating directories/documents
 - [ ] Implement WebDAV layer
   - [ ] `GET`
   - [ ] `PUT`
   - [ ] `MKCOL`
   - [ ] `DELETE`
   - [ ] `MOVE`
   - [ ] `LOCK`/`UNLOCK` (?)
 - [ ] Create HTTP interface
   - [ ] File explorer
   - [ ] Configuration menu
 - [ ] Create installation script & systemd service

## Previous Work
 - [rmrl](https://github.com/rschroll/rmrl) a python renderer for the .lines format.
 - [lines-are-beautiful](https://github.com/ax3l/lines-are-beautiful) another renderer for the .lines format, in C++.
