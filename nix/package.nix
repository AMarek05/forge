{ lib
, rustPlatform
, pkg-config
, openssl
, dbus
, fzf
, direnv
, nix
}:

rustPlatform.buildRustPackage {
  pname = "forge";
  version = "0.1.0";
  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;
  cargoSha256 = "sha256-GXQqRMeP9XGz25sJ8W7i3FHn2sksKDR8Gij6IK4dsPE=";
  buildInputs = [ pkg-config openssl dbus fzf direnv nix ];

  postInstall = ''
    mkdir -p $out/share/forge/languages
    mkdir -p $out/share/forge/docs
    cp -r languages/* $out/share/forge/languages/
    cp -r docs/* $out/share/forge/docs/
  '';
}
