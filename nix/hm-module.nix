flake:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.kubesess;
  tomlFormat = pkgs.formats.toml { };
  defaultPackage =
    flake.packages.${pkgs.stdenv.hostPlatform.system}.kubesess
      or (pkgs.callPackage ../nix/package.nix { });
in
{
  options.programs.kubesess = {
    enable = lib.mkEnableOption "kubesess — fast kubectx/kubens with session-isolated configs";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "kubesess.packages.\${system}.kubesess";
      description = "The kubesess package to install.";
    };

    enableZshIntegration = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Source kubesess shell functions (kc, kn, kcd, knd) in zsh via
        `eval "$(kubesess init zsh)"`.
      '';
    };

    enableBashIntegration = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Source kubesess shell functions in bash.";
    };

    enableFishIntegration = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Source kubesess shell functions in fish.";
    };

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = { };
      example = lib.literalExpression ''
        {
          selector = {
            height = "60%";
            layout = "reverse";
            no_sort = true;
            score_offset = 10;
            priority_rules = [
              { pattern = "^prod"; priority = 1; }
              { pattern = "^dev";  priority = 5; }
            ];
            color_rules = [
              { pattern = "^prod"; ansi = "\\u001b[32m"; }
            ];
          };
        }
      '';
      description = ''
        Contents of `~/.config/kubesess/config.toml`. Mirrors the
        `SelectorConfig` struct in `src/app_config.rs`. When empty, no file
        is written and kubesess uses its built-in defaults.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile."kubesess/config.toml" = lib.mkIf (cfg.settings != { }) {
      source = tomlFormat.generate "kubesess-config.toml" cfg.settings;
    };

    programs.zsh.initContent = lib.mkIf cfg.enableZshIntegration ''
      eval "$(${lib.getExe cfg.package} init zsh)"
    '';

    programs.bash.initExtra = lib.mkIf cfg.enableBashIntegration ''
      eval "$(${lib.getExe cfg.package} init bash)"
    '';

    programs.fish.interactiveShellInit = lib.mkIf cfg.enableFishIntegration ''
      ${lib.getExe cfg.package} init fish | source
    '';
  };
}
