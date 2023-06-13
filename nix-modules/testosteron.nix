{ config, lib, pkgs, modulesPath, ... }:

let
  num_heads = 8;
  x = 2560;
  y = 1440;
  resolution = x * y * 4 /* color depth */; /* in bytes needed */
  resolution_mb = resolution / 1024 / 1024;

  vgamem = resolution_mb * num_heads;
  ram_size = 4 * vgamem;
  vram_size = 2 * vgamem;
in
{
  imports = [
    "${modulesPath}/virtualisation/qemu-vm.nix"
    ./sway.nix
    ./zsh.nix
  ];

  nix = {
    nixPath = [
      "nixpkgs=${pkgs.path}"
    ];
    settings = {
      auto-optimise-store = true;
      cores = 2;
    };
  };

  environment.systemPackages = with pkgs; [
    curl
    git
    htop
    tmux
    vim
    fd
    ripgrep
    pciutils
  ];

  boot.loader.systemd-boot.enable = true;
  boot.extraModprobeConfig = ''
    options qxl modeset=1 num_heads=8
    # options drm debug=14
  '';

  time.timeZone = "UTC";

  networking = {
    hostName = "testosteron";
    interfaces.eth0.useDHCP = true;
    defaultGateway = null;
  };

  services.openssh = {
    enable = true;
    settings.PasswordAuthentication = true;
    settings.PermitRootLogin = "yes";
  };

  users = {
    mutableUsers = false;
    users.root.password = "";
    groups.testy = { };
    users.testy = {
      group = "testy";
      isNormalUser = true;
      password = "";
      extraGroups = [ "video" "input" ];
    };
    motd = ''
      testosteron: The testing vm for funky graphics
      Now with 100% more graphics!

      SSH: localhost:2222

      root password is "" (empty)
      testy password is "" (empty)
    '';
  };

  virtualisation = {
    diskSize = 1024;
    memorySize = 2048;
    cores = 2;
    forwardPorts = [
      { from = "host"; host.port = 2222; guest.port = 22; }
    ];
  };

  virtualisation.qemu.options = [
    # "-device qxl,vgamem_mb=128,ram_size_mb=512,vram_size_mb=256"
    "-device qxl,vgamem_mb=${toString vgamem},ram_size_mb=${toString ram_size},vram_size_mb=${toString vram_size}"
    "-machine type=q35"

    # needed for daemonize to not open a graphical display
    "-display none"
    "-daemonize -pidfile /tmp/testosteron.pid"
  ];

  system.stateVersion = "23.05";
}
