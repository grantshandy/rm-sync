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
                                                             *  file explorer web app  *
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
 - Simple installation (one statically-linked executable and a systemd service)

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
 - [ ] HTTP Server
   - [ ] WebDAV layer
     - [ ] Methods
       - [ ] `GET`
       - [ ] `PUT`
       - [ ] `MKCOL`
       - [ ] `DELETE`
       - [ ] `MOVE`
       - [ ] `LOCK`/`UNLOCK` (?)
     - [ ] Custom Directories
       - [ ] `/Trash`
       - [ ] `/Pinned`
       - [ ] `/Templates`
   - [ ] Web interface
     - [ ] File explorer
     - [ ] Configuration menu
       - [ ] RClone remote configuration (?)
       - [ ] `rclone bisync <webDAV> <cloud service>` scheduled sync for the whole filesystem.
       - [ ] `rclone sync <webDav>/file.pdf <cloud service>/file.pdf` hook for individual file updates.
   - [ ] Password Authentication/Session Management
 - [X] RClone for the reMarkable
   - [X] Cross-compile RCLone (`nix build nixpkgs#pkgsCross.remarkable2.pkgsStatic.rclone`)
   - [ ] Build `librclone` and statically link (?)
 - [ ] Create installation script & systemd service

## Previous Work
 - [reMarkable .lines File Format](https://plasma.ninja/blog/devices/remarkable/binary/format/2017/12/26/reMarkable-lines-file-format.html) (Axel Huebl)
 - [rmrl](https://github.com/rschroll/rmrl) a python renderer for the .lines format.
 - [lines-are-beautiful](https://github.com/ax3l/lines-are-beautiful) another renderer for the .lines format, in C++.
