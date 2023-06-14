{ craneLib, pkgs, ... }:

let
  testsFilter = path: _type:
    (builtins.match ''^/nix/store/[^/]+/tests(|.*)'' path != null);
  nextestTomlFilter = path: _type:
    (builtins.match ''.*/nextest.toml'' path != null);
  testsOrCargo = path: type:
    (testsFilter path type) || (nextestTomlFilter path type) || (craneLib.filterCargoSources path type);

  commonArgs = {
    src = pkgs.lib.cleanSourceWith {
      src = craneLib.path ./..;
      filter = testsOrCargo;
    };
    cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./../Cargo.lock; };
    doCheck = false;
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // { });

  cargoNextestArchive = pkgs.callPackage ./cargoNextestArchive.nix {
    inherit craneLib;
    cargo-nextest = pkgs.cargo-nextest;
  };

  shikane = craneLib.buildPackage (commonArgs // {
    inherit cargoArtifacts;
  });

  shikane-clippy = craneLib.cargoClippy (commonArgs // {
    inherit cargoArtifacts;
  });

  shikane-nextest-archive = cargoNextestArchive (commonArgs // {
    inherit cargoArtifacts;
  });


  shikane-docs = pkgs.stdenv.mkDerivation {
    name = "shikane-docs";
    src = ./..;
    nativeBuildInputs = with pkgs; [ pandoc installShellFiles ];
    buildPhase = ''
      runHook preBuild
      bash scripts/build-docs.sh man ${shikane.version}
      runHook postBuild
    '';
    installPhase = ''
      runHook preInstall
      installManPage build/*
      runHook postInstall
    '';
  };
in
{
  default = pkgs.symlinkJoin {
    name = "shikane";
    paths = [ shikane shikane-docs ];
  };
  shikane = shikane;
  shikane-docs = shikane-docs;
  shikane-clippy = shikane-clippy;
  shikane-nextest-archive = shikane-nextest-archive;
}
