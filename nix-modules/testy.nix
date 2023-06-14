{ config, inputs, pkgs, ... }:

let
  archive = inputs.self.packages.${pkgs.system}.shikane-nextest-archive;
  archiveFile = "${archive}/archive.tar.zst";
  cargoSource = archive.cargoArtifacts.src;
  nextestWorkspace = "/tmp/nextest-workspace";
  nextestArgs = "run --archive-file ${archiveFile} --workspace-remap ${nextestWorkspace}";

  copySource = pkgs.writeShellScript "copy-source"
    ''
      set -euo pipefail
      if [[ -d ${nextestWorkspace} ]]; then
        chmod +rw ${nextestWorkspace} --recursive
      fi
      rm -rf ${nextestWorkspace}
      mkdir -p ${nextestWorkspace}
      cp -r ${cargoSource}/* ${nextestWorkspace}/
      cp -r ${archive}/tests/ ${nextestWorkspace}/
      cp -r ${archive}/.config/ ${nextestWorkspace}/
    '';

  nextestWrapper = pkgs.writeShellScript "nextest-wrapper"
    ''
      ${pkgs.cargo-nextest}/bin/cargo-nextest nextest ${nextestArgs} 2>&1 | ${pkgs.nmap}/bin/ncat 10.0.2.2 2233
    '';
in
{
  security.sudo.enable = true;
  security.sudo.extraConfig = ''
    testy ALL=(ALL:ALL) NOPASSWD: ALL
  '';

  # See https://github.com/NixOS/nixpkgs/issues/3702
  systemd.services.linger-login = {
    enable = true;
    description = "Enable user lingering for testy";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${pkgs.systemd}/bin/loginctl enable-linger testy";
    };
  };

  boot.blacklistedKernelModules = [ "bochs" ];

  users.users.testy.packages = [ pkgs.cargo-nextest ];
  systemd.user.services.cargo-nextest = {
    enable = true;
    description = "cargo nextest executor";
    wantedBy = [ "default.target" ];
    after = [
      "network-online.target"
      "seatd.service"
    ];
    wants = [
      "network-online.target"
    ];
    serviceConfig = {
      ExecStartPre = "${copySource}";
      ExecStart = "${nextestWrapper}";
      ExecStopPost = "/run/wrappers/bin/sudo systemctl --no-block poweroff";
    };
    environment = {
      PATH = pkgs.lib.mkForce "/run/wrappers/bin:/home/testy/.nix-profile/bin:/etc/profiles/per-user/testy/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin";
      CARGO_TERM_COLOR = "always";
    };
  };
}
