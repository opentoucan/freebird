let
  # Last updated: 2/26/21. Update as necessary from https://status.nixos.org/...
  pkgs = import (fetchTarball("https://github.com/NixOS/nixpkgs/archive/d70bd19e0a38ad4790d3913bf08fcbfc9eeca507.tar.gz")) {};

  # Rolling updates, not deterministic.
  # pkgs = import (fetchTarball("channel:nixpkgs-unstable")) {};
in pkgs.mkShell {
  buildInputs = [
    pkgs.cargo
    pkgs.rustc
    pkgs.rustfmt
    pkgs.cmake

    # Necessary for the openssl-sys crate:
    pkgs.openssl
    pkgs.pkg-config

    pkgs.llvmPackages.bintools
    pkgs.gcc
    pkgs.clippy
    pkgs.libopus
    pkgs.yt-dlp

    #Misc packages for local dev
    pkgs.dasel
  ];

  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  shellHook = ''
    export DISCORD_TOKEN=""
  '';
}
