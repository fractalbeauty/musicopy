{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, fenix, ... }:
    let
      eachSupportedSystem = nixpkgs.lib.genAttrs nixpkgs.lib.systems.flakeExposed;
    in
    {
      devShells = eachSupportedSystem (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfree = true;
            config.android_sdk.accept_license = true;
          };
          f = fenix.packages.${system};

          rustToolchain = f.combine [
            f.stable.defaultToolchain
            f.stable.rust-src
            f.stable.llvm-tools-preview
            f.targets.aarch64-linux-android.stable.rust-std
            f.targets.armv7-linux-androideabi.stable.rust-std
            f.targets.i686-linux-android.stable.rust-std
            f.targets.x86_64-linux-android.stable.rust-std
          ];

          androidComposition = pkgs.androidenv.composeAndroidPackages {
            buildToolsVersions = [
              "34.0.0"
            ];
            platformVersions = [
              "33"
              "34"
              "35"
              "latest"
            ];
            includeNDK = true;
          };
          lastBuildTools = pkgs.lib.lists.last androidComposition.build-tools;
        in
        {
          default = pkgs.mkShell rec {
            nativeBuildInputs = [
              rustToolchain
              pkgs.cargo-ndk
              pkgs.cargo-nextest
              pkgs.cargo-llvm-cov
              pkgs.just
              pkgs.jdk
            ];

            JAVA_HOME = "${pkgs.jdk}";
            ANDROID_HOME = "${androidComposition.androidsdk}/libexec/android-sdk";
            GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${ANDROID_HOME}/build-tools/${lastBuildTools.version}/aapt2";
          };
        }
      );
    };
}
