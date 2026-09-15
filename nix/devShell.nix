{
  inputs,
  callPackage,
  mkShell,
  writeShellScriptBin,
  ...
}:
(
  let
    axolotl-git= callPackage ./axolotl-git { inherit inputs; };
    update-gradle-deps = writeShellScriptBin "update-gradle-deps" ''
      ${axolotl-git.mitmCache.updateScript}
    '';
  in
    mkShell {
      packages = [
        update-gradle-deps
      ];
    }
)
