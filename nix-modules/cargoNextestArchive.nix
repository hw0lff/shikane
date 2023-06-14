# Stolen from craneLib documentation
{ cargo-nextest, craneLib }:

{ cargoArtifacts
, cargoNextestArchiveExtraArgs ? "" # Arguments that are generally useful default
, cargoExtraArgs ? "" # Other cargo-general flags (e.g. for features or targets)
, ...
}@origArgs:
let
  # Clean the original arguments for good hygiene (i.e. so the flags specific
  # to this helper don't pollute the environment variables of the derivation)
  args = builtins.removeAttrs origArgs [
    "cargoNextestArchiveExtraArgs"
    "cargoExtraArgs"
  ];

  cargoProfile = "$" + "{CARGO_PROFILE:+--cargo-profile $CARGO_PROFILE}";
  archiveArgs = "--archive-format tar-zst --archive-file $out/archive.tar.zst";

  nextestArgs = "archive ${cargoProfile} ${cargoExtraArgs} ${archiveArgs} ${cargoNextestArchiveExtraArgs}";
in
craneLib.mkCargoDerivation (args // {
  # Additional overrides we want to explicitly set in this helper

  # Require the caller to specify cargoArtifacts we can use
  inherit cargoArtifacts;

  # A suffix name used by the derivation, useful for logging
  pnameSuffix = "-nextest-archive";

  # Set the cargo command we will use and pass through the flags
  buildPhaseCargoCommand = ''
    mkdir -p $out
    cargo nextest --version
    cargo nextest ${nextestArgs}
    cp -r .config/ $out
    cp -r tests/ $out
  '';

  # Append the `cargo-nextest` package to the nativeBuildInputs set by the
  # caller (or default to an empty list if none were set)
  nativeBuildInputs = (args.nativeBuildInputs or [ ]) ++ [ cargo-nextest ];
})
