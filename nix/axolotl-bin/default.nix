{
  inputs,
  fetchurl,
  libarchive,
  stdenv,
  ...
}:
stdenv.mkDerivation rec {
  pname = "axolotl-bin";
  version = "1.9.6";
  src = (
    let
      base = "https://github.com/Mystic-Stars/Axolotl/releases/download/v${version}";
      debs = {
        x86_64-linux = {
          url = "${base}/Axolotl.Launcher_${version}_amd64.deb";
          hash = "sha256:82b38a3ac82442844ea22f723c5a4bff87c8b977257dbe7698d49c0efda350c2";
        };
        aarch64-linux = {
          url = "${base}/Axolotl.Launcher_${version}_arm64.deb";
          hash = "sha256:66d5bc47cf31f779be86e0b28d0da62b6db3a3a7685b3339823f25fdf31e307d";
        };
      };
      sys = stdenv.hostPlatform.system;
      tar = debs.${sys} or (throw "Unsupported system: ${sys}");
    in
      fetchurl tar
  );
  nativeBuildInputs = [ libarchive ];
  unpackPhase = ''
    runHook preUnpack
    mkdir -p "$out/bin"
    bsdtar -xf "$src" "data.tar.gz"
    bsdtar -xf "data.tar.gz" -C "$out/bin" --strip-components=2 "usr/bin/Axolotl Launcher"
    runHook postUnpack
  '';
  # installPhase = ''
  #   runHook preInstall
  #   chmod +x "$out/bin/Axolotl Launcher"
  #   runHook postInstall
  # '';
}
