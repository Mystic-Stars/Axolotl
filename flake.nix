{
  description = "Axolotl Launcher";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    self.submodules = true;
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      packageSet = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };
          pnpm = pkgs.pnpm_10.overrideAttrs (_: rec {
            version = "10.33.2";
            src = pkgs.fetchurl {
              url = "https://registry.npmjs.org/pnpm/-/pnpm-${version}.tgz";
              hash = "sha512-qQ+vb+6rca1sblf5Tg/hoS9dzCLNdU20CulZPraj4LaxLjVAIYuzeuCDQEsfLObbKkEh6XmCm0r/lLmfSdoc+A==";
            };
          });
          axolotl = pkgs.callPackage ./nix/package.nix {
            src = self;
            inherit pnpm rustPlatform;
            nodejs = pkgs.nodejs_24;
          };
        in
        {
          inherit
            axolotl
            pkgs
            pnpm
            rustPlatform
            rustToolchain
            ;
        }
      );
    in
    {
      packages = forAllSystems (
        system:
        let
          inherit (packageSet.${system}) axolotl;
        in
        {
          axolotl-launcher = axolotl;
          default = axolotl;
          gradle-deps-update = axolotl.passthru.gradle-deps-update;
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = packageSet.${system}.pkgs.lib.getExe packageSet.${system}.axolotl;
        };
      });

      checks = forAllSystems (system: {
        default = packageSet.${system}.axolotl;
      });

      devShells = forAllSystems (
        system:
        let
          inherit (packageSet.${system})
            pkgs
            pnpm
            rustToolchain
            ;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo-nextest
              git
              jdk17
              nodejs_24
              patchelf
              pkg-config
              pnpm
              rustToolchain
              xdg-utils
            ];

            buildInputs = with pkgs; [
              glib
              glib-networking
              libayatana-appindicator
              librsvg
              openssl
              webkitgtk_4_1
            ];

            JAVA_HOME = "${pkgs.jdk17}";
            GIO_MODULE_DIR = "${pkgs.glib-networking}/lib/gio/modules";
          };
        }
      );
    };
}
