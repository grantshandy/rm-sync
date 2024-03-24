{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    gomod2nix = {
      url = "github:nix-community/gomod2nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, flake-utils, gomod2nix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ gomod2nix.overlays.default ];
        };

        goEnv = pkgs.mkGoEnv { 
          pwd = ./.;
          modules = ./gomod2nix.toml;
        };

        # pass in custom nixpkgs for cross-compilation/static linking
        buildPackage = crossPkgsStatic: crossPkgsStatic.buildGoApplication rec {
          pname = "rm-cloudshim";
          version = "0.1.0";
          src = ./.;
          modules = ./gomod2nix.toml;

          nativeBuildInputs = [ crossPkgsStatic.musl ];

          CGO_ENABLED = 1;
          ldflags = [
            "-linkmode external"
            "-extldflags '-static -L${crossPkgsStatic.musl}/lib'"
            "-s"
            "-w"
          ];
        };
      in
      {
        packages = {
          default = buildPackage pkgs.pkgsStatic;
          remarkable2 = buildPackage pkgs.pkgsCross.remarkable2.pkgsStatic;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rclone gopls go-tools delve runc pkgs.gomod2nix goEnv
          ];
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
