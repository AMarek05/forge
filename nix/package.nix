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

  buildInputs = [ pkg-config openssl dbus fzf direnv nix ];

  postInstall = ''
    mkdir -p $out/share/forge/languages
    mkdir -p $out/share/forge/includes
    cp -r languages/* $out/share/forge/languages/
    cp -r includes/*  $out/share/forge/includes/
    cp completions/zsh/_forge $out/share/zsh/site-functions/
  '';
}
