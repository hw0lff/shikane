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

  outputs = { self, nixpkgs, crane, flake-utils, fenix, ... }@inputs:
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
          shikane-pkg = pkgs.callPackage ./nix-modules/shikane.nix { inherit craneLib pkgs; };
          testosteron-vm = self.nixosConfigurations.testosteron.config.system.build.vm;
        in
        rec
        {
          packages = {
            default = shikane-pkg.default;
            shikane = shikane-pkg.shikane;
            shikane-docs = shikane-pkg.shikane-docs;
            testosteron-vm = testosteron-vm;
          };

          apps.default = apps.test;
          apps.test = {
            type = "app";
            program = "${self.packages.${system}.testosteron-vm}/bin/run-testosteron-vm";
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

      nixosConfigurations.testosteron = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = { inherit inputs; };
        modules = [
          ./nix-modules/testosteron.nix
        ];
      };
    };
}
