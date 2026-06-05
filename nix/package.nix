{ lib, rustPlatform }:

let
  cargoToml = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  doCheck = false;

  meta = with lib; {
    description = "Fast kubectx/kubens replacement with session-isolated configs";
    homepage = "https://github.com/ramilito/kubesess";
    license = licenses.mit;
    mainProgram = "kubesess";
    platforms = platforms.unix;
  };
}
