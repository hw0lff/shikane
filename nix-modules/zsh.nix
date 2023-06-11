{ pkgs, ... }:
{
  users.defaultUserShell = pkgs.zsh;
  programs.zsh.enable = true;
  environment.etc.zshrc.text = ''
    source ${pkgs.agdsn-zsh-config}/etc/zsh/zshrc
  '';
}
