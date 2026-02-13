{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];

          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "llvm-tools-preview" ];
            targets = [
              "aarch64-linux-android"
              "armv7-linux-androideabi"
              "i686-linux-android"
              "x86_64-linux-android"
            ];
          };

          nativeBuildInputs = [
            rustToolchain
            pkgs.cargo-ndk
            pkgs.cargo-nextest
            pkgs.cargo-llvm-cov
            pkgs.just
          ];
          buildInputs = [];
        in
        {
          devShells.default = pkgs.mkShell rec {
            inherit nativeBuildInputs buildInputs;
          };
        }
      );
}
