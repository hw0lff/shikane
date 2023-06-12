{ config, pkgs, ... }:
let
  touchy-testy-zshrcy = pkgs.writeShellScript "touchy-testy-zshrcy"
    ''
      touch ${config.users.users.testy.home}/.zshrc
      chown testy:testy ${config.users.users.testy.home}/.zshrc
    '';
in
{
  users.defaultUserShell = pkgs.zsh;
  programs.zsh.enable = true;
  environment.etc.zshrc.text = ''
    source ${pkgs.agdsn-zsh-config}/etc/zsh/zshrc
  '';

  systemd.services.testy-zshrc = {
    enable = true;
    description = "touch ${config.users.users.testy.home}/.zshrc to stop the zsh greeting at first login";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ConditionPathExists = "!${config.users.users.testy.home}/.zshrc";
      ExecStart = "${touchy-testy-zshrcy}";
    };
  };
}
