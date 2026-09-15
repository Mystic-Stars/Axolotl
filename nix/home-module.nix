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
      prebuilt = mkOption {
        type = with lib.types; bool;
        default = true;
        description = ''
        	Whether to use prebuilt version of Axolotl Launcher.
        	When set to false, Axolotl Launcher will be built from source locally.
        	When set to true, Axolotl Launcher will be downloaded from GitHub.
        '';
      	example = false;
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
      home.packages = [ (callPackage ./package.nix { inherit inputs launchEnv prebuilt; }) ];
    };
  }
