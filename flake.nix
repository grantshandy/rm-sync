{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "utils";
      };
    };
    htmx = {
      url = "https://unpkg.com/htmx.org@1.9.11/dist/htmx.min.js";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, rust-overlay, crane, htmx, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        CARGO_BUILD_TARGET = "armv7-unknown-linux-musleabihf";
        HTMX = "${htmx}";

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ CARGO_BUILD_TARGET ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        rmPkgs = pkgs.pkgsCross.remarkable2.pkgsStatic;


        crate = craneLib.buildPackage rec {
          inherit HTMX CARGO_BUILD_TARGET;

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          buildInputs = [ rmPkgs.stdenv.cc ];
          doCheck = false;

          TARGET_CC = "${rmPkgs.stdenv.cc.targetPrefix}cc";
          CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER = TARGET_CC;
        };
      in
      {
        packages.default = crate;

        devShells.default = pkgs.mkShell rec {
          inherit HTMX;
          RUST_SRC_PATH = "${devToolchain}/lib/rustlib/src/rust/library";
          RUST_LOG = "debug";

          devToolchain = rustToolchain.override { extensions = [ "rust-analyzer" "rust-src" ]; };
          buildInputs = with pkgs; [
            devToolchain
            cargo-watch
            cargo-expand
            dave
          ];
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
