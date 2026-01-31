{ pkgs ? import <nixpkgs> { } }:

let
  rustOverlay = import (builtins.fetchTarball https://github.com/oxalica/rust-overlay/archive/master.tar.gz);
  pkgsWithOverlay = import <nixpkgs> { overlays = [ rustOverlay ]; };
in 
pkgsWithOverlay.mkShell {
  buildInputs = with pkgs; [
    pkgsWithOverlay.rust-bin.stable.latest.complete    # Latest stable Rust
    gcc
  ];
  shellHook = ''
    export RUSTUP_TOOLCHAIN=stable
    export RUST_SRC_PATH="${pkgsWithOverlay.rust-bin.stable.latest.rust-src}";
  '';
}

