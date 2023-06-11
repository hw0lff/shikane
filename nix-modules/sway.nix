{ pkgs, ... }:
{
  programs.sway.enable = true;
  programs.sway.extraPackages = with pkgs; [
    alacritty
    foot
    seatd
  ];

  systemd.services.seatd = {
    description = "seatd for sway";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${pkgs.seatd}/bin/seatd -g video";
    };
  };
}
