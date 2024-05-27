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
      bash scripts/build-docs.sh html ${shikane.version}
      runHook postBuild
    '';
    installPhase = ''
      runHook preInstall
      installManPage build/man/*
      mkdir -p $out/share/doc/shikane/html
      mv build/html/* $out/share/doc/shikane/html
      mv README.md $out/share/doc/shikane/
      mv CHANGELOG.md $out/share/doc/shikane/
      mkdir -p $out/share/licenses/shikane/
      mv LICENSE $out/share/licenses/shikane/
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
