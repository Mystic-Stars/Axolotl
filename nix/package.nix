{
  lib,
  stdenv,
  src,
  addDriverRunpath,
  alsa-lib,
  cacert,
  cargo-tauri,
  desktop-file-utils,
  fetchPnpmDeps,
  flite,
  glib,
  glib-networking,
  gradle_9,
  gsettings-desktop-schemas,
  jdk8,
  jdk17,
  jdk21,
  jdk25,
  libGL,
  libayatana-appindicator,
  libjack2,
  libpulseaudio,
  librsvg,
  libx11,
  libxcursor,
  libxext,
  libxrandr,
  libxxf86vm,
  makeShellWrapper,
  nodejs,
  openssl,
  patchelf,
  pipewire,
  pkg-config,
  pnpm,
  pnpmConfigHook,
  replaceVars,
  runCommand,
  rustPlatform,
  symlinkJoin,
  turbo,
  udev,
  webkitgtk_4_1,
  wrapGAppsHook3,
  xdg-utils,
  xrandr,
}:

let
  frontendPackage = builtins.fromJSON (builtins.readFile ../apps/app-frontend/package.json);
  gradle = gradle_9.override { java = jdk17; };
  gradleExe =
    runCommand "gradle-exe-wrapper-${gradle.version}" { nativeBuildInputs = [ makeShellWrapper ]; }
      ''
        makeShellWrapper ${lib.getExe gradle} $out \
          --add-flags "\''${NIX_GRADLEFLAGS_COMPILE:-}"
      '';

  unwrapped = rustPlatform.buildRustPackage (finalAttrs: {
    pname = "axolotl-launcher-unwrapped";
    version = frontendPackage.version;
    inherit src;

    patches = [
      (replaceVars ./gradle-from-path.patch {
        gradle = gradleExe;
      })
    ];

    postPatch = ''
      test -f apps/app-frontend/src/data/about/contributors.json
      substituteInPlace apps/app-frontend/package.json \
        --replace-fail \
          'pnpm contributors:sync && vue-tsc --noEmit && vite build' \
          'vue-tsc --noEmit && vite build'
    '';

    cargoHash = "sha256-+qF2afe13Flk2uYFTto6ebn25gKB/aHQcEJJkX/WpC8=";

    mitmCache = gradle.fetchDeps {
      pkg = finalAttrs.finalPackage;
      inherit (finalAttrs) pname;
      attrPath = null;
      data = ./gradle-deps.json;
    };

    pnpmDeps = fetchPnpmDeps {
      inherit (finalAttrs) pname version src;
      inherit pnpm;
      fetcherVersion = 4;
      hash = "sha256-Lebms4fNJK2VP3Ef/ExfhaIJSN2YEz7c1KqpDLZVziI=";
    };

    nativeBuildInputs = [
      cacert
      cargo-tauri.hook
      desktop-file-utils
      gradle
      jdk17
      nodejs
      patchelf
      pkg-config
      pnpm
      pnpmConfigHook
    ];

    buildInputs = [
      glib-networking
      libayatana-appindicator
      librsvg
      openssl
      webkitgtk_4_1
    ];

    gradleFlags = [
      "-Dfile.encoding=utf-8"
      "--no-configuration-cache"
      "-x"
      "spotlessJava"
    ];
    dontUseGradleBuild = true;
    dontUseGradleCheck = true;

    cargoTestFlags = [
      "--package"
      "theseus_gui"
    ];

    env.TURBO_BINARY_PATH = lib.getExe turbo;

    preGradleUpdate = ''
      cd packages/app-lib/java
    '';
    gradleUpdateTask = "nixDownloadDeps authlibInjector";

    preBuild = ''
      local nixGradleFlags=()
      concatTo nixGradleFlags gradleFlags gradleFlagsArray
      export NIX_GRADLEFLAGS_COMPILE="''${nixGradleFlags[@]}"
    '';

    passthru = {
      inherit gradle;
    };

    meta = {
      description = "Cross-platform Minecraft launcher from Axolotl Launcher";
      homepage = "https://www.axlmc.org";
      license = lib.licenses.gpl3Only;
      mainProgram = "Axolotl Launcher";
      platforms = lib.platforms.linux;
      sourceProvenance = with lib.sourceTypes; [
        fromSource
        binaryBytecode
      ];
    };
  });

  jdks = [
    jdk8
    jdk17
    jdk21
    jdk25
  ];
in
symlinkJoin {
  pname = "axolotl-launcher";
  inherit (unwrapped) version;

  paths = [ unwrapped ];
  strictDeps = true;

  nativeBuildInputs = [
    glib
    wrapGAppsHook3
  ];

  buildInputs = [
    glib-networking
    gsettings-desktop-schemas
  ];

  runtimeDependencies = lib.makeLibraryPath [
    addDriverRunpath.driverLink
    libGL
    libx11
    libxcursor
    libxext
    libxrandr
    libxxf86vm
    (lib.getLib stdenv.cc.cc)
    flite
    alsa-lib
    libjack2
    libpulseaudio
    pipewire
    udev
  ];

  postBuild = ''
    gappsWrapperArgs+=(
      --prefix PATH : ${lib.makeSearchPath "bin/java" jdks}
      --prefix PATH : ${
        lib.makeBinPath [
          xrandr
          xdg-utils
        ]
      }
      --set LD_LIBRARY_PATH $runtimeDependencies
    )

    glibPostInstallHook
    gappsWrapperArgsHook
    wrapGApp "$out/bin/Axolotl Launcher"
  '';

  passthru = {
    inherit unwrapped;
    gradle-deps-update = unwrapped.mitmCache.updateScript;
  };

  meta = unwrapped.meta;
}
