{
  description = "A simple project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, ... }:
    let
      # System types to support.
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];

      # Rust nightly version.
      nightlyVersion = "2022-12-04";
    in
    flake-utils.lib.eachSystem supportedSystems (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustNightly = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
          extensions = [ "rust-src" "rust-analyzer-preview" ];
          targets = [ "x86_64-unknown-linux-gnu" ];
        };
        craneLib = (crane.mkLib pkgs).overrideToolchain rustNightly;

        # src = craneLib.cleanCargoSource ./.;
        src = ./.;

        meeseeksPackage = craneLib.buildPackage {
          inherit src;
          nativeBuildInputs = with pkgs; [
            pkg-config
            openssl
            gcc
            protobuf
          ];
          doCheck = false;
        };
        meeseeksDockerImage = pkgs.dockerTools.buildImage {
          name = "meeseeks";
          tag = "latest";
          fromImageName = "alpine";
          fromImageTag = "latest";

          copyToRoot = pkgs.buildEnv {
            name = "image-root";
            paths = [ meeseeksPackage pkgs.bash pkgs.coreutils-full ];
            pathsToLink = [ "/bin" ];
          };
        };
      in
      {
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustNightly
            protobuf
            grpcurl
            pkg-config
            openssl
            python310
            python310Packages.virtualenv
            python310Packages.pandas
            stdenv.cc.cc.lib
          ];
        };

        packages = {
          default = meeseeksPackage;
          meeseeksDocker = meeseeksDockerImage;
        };
      });
}
