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
          testosteron = self.nixosConfigurations.testosteron;
          testosteron-vm = testosteron.config.system.build.vm;
          testosteron-vm-wrapper = pkgs.writeScript "testosteron-vm-wrapper"
            ''
              # kill old VM
              kill $(cat /tmp/testosteron.pid 2>/dev/null) 2>/dev/null

              set -euo pipefail
              echo testosteron: The testing vm for funky graphics
              echo Now with 100% more graphics!
              echo
              echo 'SSH:       localhost:2222'
              echo 'test-log:  localhost:2233'
              echo
              echo root password is \"\" '(empty)'
              echo testy password is \"\" '(empty)'
              echo
              echo QEMU will create a pidfile at /tmp/testosteron.pid
              echo In case of problems, kill it with:
              echo
              echo kill '$(cat /tmp/testosteron.pid)'
              echo

              echo 'Starting the VM...'
              ${testosteron-vm}/bin/run-testosteron-vm

              echo 'Watching logs...'
              echo '========================='
              ${pkgs.nmap}/bin/ncat localhost 2233 --listen
            '';
        in
        rec
        {
          packages = {
            default = shikane-pkg.default;
            shikane = shikane-pkg.shikane;
            shikane-docs = shikane-pkg.shikane-docs;
            shikane-nextest-archive = shikane-pkg.shikane-nextest-archive;
            testosteron-vm = testosteron-vm;
            testosteron-vm-wrapper = testosteron-vm-wrapper;
          };

          apps.default = apps.test;
          apps.test = {
            type = "app";
            program = "${self.packages.${system}.testosteron-vm-wrapper}";
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
