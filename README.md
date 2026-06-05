# kubesess

Fast `kubectx`/`kubens` replacement with session-isolated kubeconfigs.

## Install via Nix flake

The flake exposes `packages.<system>.kubesess`, an overlay, a devshell, and a
home-manager module.

### Try it without installing

```sh
nix run github:ramilito/kubesess -- --help
```

### Add to a flake-based system

```nix
{
  inputs.kubesess.url = "github:ramilito/kubesess";
  # optional but recommended:
  inputs.kubesess.inputs.nixpkgs.follows = "nixpkgs";
}
```

Then either pull in the package directly:

```nix
environment.systemPackages = [ inputs.kubesess.packages.${pkgs.system}.default ];
```

…or apply the overlay so `pkgs.kubesess` is available everywhere:

```nix
nixpkgs.overlays = [ inputs.kubesess.overlays.default ];
```

## Home-manager module

Import `homeManagerModules.kubesess` (alias: `homeManagerModules.default`):

```nix
{ inputs, ... }: {
  imports = [ inputs.kubesess.homeManagerModules.default ];

  programs.kubesess = {
    enable = true;

    # Shell integrations are on by default — toggle off if not wanted.
    enableZshIntegration  = true;
    enableBashIntegration = false;
    enableFishIntegration = false;

    # Rendered to ~/.config/kubesess/config.toml. Mirrors SelectorConfig in
    # src/app_config.rs. Omit `settings` entirely to use built-in defaults.
    settings.selector = {
      height       = "60%";
      layout       = "reverse";
      no_sort      = true;
      score_offset = 10;

      priority_rules = [
        { pattern = "^prod"; priority = 1; }
        { pattern = "^dev";  priority = 5; }
      ];

      color_rules = [
        { pattern = "^prod"; ansi = "\\u001b[32m"; }  # green
        { pattern = "^dev";  ansi = "\\u001b[33m"; }  # yellow
      ];
    };
  };
}
```

### Module options

| Option | Default | Purpose |
|---|---|---|
| `enable` | `false` | Master switch |
| `package` | flake's `kubesess` | Override the package |
| `enableZshIntegration` | `true` | `eval "$(kubesess init zsh)"` in zsh init |
| `enableBashIntegration` | `true` | Same for bash |
| `enableFishIntegration` | `true` | `kubesess init fish \| source` for fish |
| `settings` | `{}` | TOML-compatible attrset rendered to `config.toml` |

## Devshell

```sh
nix develop
```

Provides `cargo`, `rustc`, `clippy`, `rustfmt`, `rust-analyzer`.
