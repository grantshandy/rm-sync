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
  };

  outputs = { self, nixpkgs, utils, rust-overlay, crane, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        CARGO_BUILD_TARGET = "armv7-unknown-linux-musleabihf";

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ CARGO_BUILD_TARGET ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        rmPkgs = pkgs.pkgsCross.remarkable2.pkgsStatic;

        crate = craneLib.buildPackage {
          inherit CARGO_BUILD_TARGET;

          src = craneLib.cleanCargoSource (craneLib.path ./.);
          buildInputs = [ rmPkgs.stdenv.cc ];
          doCheck = false;

          CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER = "${rmPkgs.stdenv.cc.targetPrefix}cc";
        };
      in
      {
        packages.default = crate;

        devShells.default = pkgs.mkShell rec {
          devToolchain = rustToolchain.override { extensions = [ "rust-analyzer" "rust-src" ]; };
          buildInputs = with pkgs; [
            devToolchain
            rmPkgs.stdenv.cc
          ];

          RUST_SRC_PATH = "${devToolchain}/lib/rustlib/src/rust/library";
          CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER = "${rmPkgs.stdenv.cc.targetPrefix}cc";
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
