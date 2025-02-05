{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.mkLib pkgs;

        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          buildInputs = [
            pkgs.gst_all_1.gstreamer
            pkgs.gst_all_1.gst-plugins-base
            pkgs.glib

            # pkgs.alsa-lib
            pkgs.pkg-config
          ];

          nativeBuildInputs = [
            pkgs.pkg-config
            # pkgs.pkg-config
            # pkgs.alsa-lib
          ];
        };

        qobuz-player = craneLib.buildPackage (commonArgs // {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        });
      in
      {
        checks = {
          inherit qobuz-player;
        };

        packages.default = qobuz-player;

        apps.default = flake-utils.lib.mkApp {
          drv = qobuz-player;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          packages = [
            pkgs.cargo-machete
            pkgs.sqlx-cli
            # pkgs.pkg-config
          ];

          DATABASE_URL = "sqlite:///tmp/data.db";
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        };
      });
}
