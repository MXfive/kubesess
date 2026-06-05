{
  description = "kubesess — fast kubectx/kubens with session-isolated configs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    let
      hmModule = import ./nix/hm-module.nix self;
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        kubesess = pkgs.callPackage ./nix/package.nix { };
      in
      {
        packages = {
          inherit kubesess;
          default = kubesess;
        };

        apps.default = {
          type = "app";
          program = "${kubesess}/bin/kubesess";
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc clippy rustfmt rust-analyzer ];
        };
      }
    ) // {
      overlays.default = final: prev: {
        kubesess = final.callPackage ./nix/package.nix { };
      };

      homeManagerModules.kubesess = hmModule;
      homeManagerModules.default = hmModule;
    };
}
