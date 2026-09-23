{
  inputs,
  legacyPackages,
  ...
}:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib) mkEnableOption mkOption mkIf;
  callPackage = legacyPackages.${pkgs.stdenv.hostPlatform.system}.callPackage;
in
  {
    options.programs.axolotl-launcher = {
      enable = mkEnableOption "Axolotl Launcher";
      launchEnv = mkOption {
        type = with lib.types; attrsOf anything;
        default = {};
        description = ''
          Environment variables or flags to be passed to Axolotl.
        '';
        example = {
          WEBKIT_DISABLE_DMABUF_RENDERER = 1;
        };
      };
      jres = mkOption {
        type = with lib.types; listOf package;
        default = [];
        description = ''
          (WIP)
          A list of packages of JREs/JDKs to be written into the Java list.
        '';
        example = [ pkgs.jre8 ];
      };
    };

    config = with config.programs.axolotl-launcher; mkIf enable {
      home.packages = [ (callPackage ./package.nix { inherit inputs launchEnv; }) ];
    };
  }
