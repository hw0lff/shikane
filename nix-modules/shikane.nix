{ craneLib, pkgs, ... }:

let
  commonArgs = {
    src = craneLib.cleanCargoSource (craneLib.path ./..);
    cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = ./../Cargo.lock; };
    doCheck = false;
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // { });

  shikane-clippy = craneLib.cargoClippy (commonArgs // {
    inherit cargoArtifacts;
  });

  shikane = craneLib.buildPackage (commonArgs // {
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
}
