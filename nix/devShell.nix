{
  inputs,
  callPackage,
  mkShell,
  writeShellScriptBin,
  ...
}:
(
  let
    axolotl = callPackage ./axolotl.nix { inherit inputs; };
    update-gradle-deps = writeShellScriptBin "update-gradle-deps" ''
      ${axolotl.mitmCache.updateScript}
    '';
  in
    mkShell {
      packages = [
        update-gradle-deps
      ];
    }
)
