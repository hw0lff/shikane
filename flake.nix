{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, fenix, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };

          fenixChannel = fenix.packages.${system}.stable;

          fenixToolchain = (fenixChannel.withComponents [
            "rustc"
            "cargo"
            "rustfmt"
            "clippy"
            "rust-analysis"
            "rust-src"
            "llvm-tools-preview"
          ]);
          craneLib = crane.lib.${system}.overrideToolchain fenixToolchain;
        in
        rec
        {
          packages = rec {
            default = pkgs.symlinkJoin {
              name = "shikane";
              paths = [ packages.shikane packages.shikane-docs ];
            };
            shikane = craneLib.buildPackage {
              name = "shikane";
              src = craneLib.cleanCargoSource ./.;
              doCheck = false;
              cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./Cargo.lock; };
            };
            shikane-docs = pkgs.stdenv.mkDerivation {
              name = "shikane-docs";
              src = ./.;
              nativeBuildInputs = with pkgs; [ pandoc installShellFiles ];
              buildPhase = ''
                runHook preBuild
                bash scripts/build-docs.sh man ${packages.shikane.version}
                runHook postBuild
              '';
              installPhase = ''
                runHook preInstall
                installManPage build/*
                runHook postInstall
              '';
            };
          };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs = (with packages.shikane; nativeBuildInputs ++ buildInputs) ++ [ fenixToolchain ];
            RUST_SRC_PATH = "${fenixChannel.rust-src}/lib/rustlib/src/rust/library";
          };
        }) // {
      overlays.default = (final: prev: {
        inherit (self.packages.${prev.system})
          shikane;
      });
    };
}
