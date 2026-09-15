{
  inputs,
  makeDesktopItem,
  ...
}:
makeDesktopItem {
  categories = [ "Game" ];
  desktopName = "Axolotl Launcher";
  exec = "axolotl-launcher";
  icon = "${../apps/app/icons/icon.png}";
  mimeTypes = [
    "application/x-modrinth-modpack+zip"
    "x-scheme-handler/axolotl"
  ];
  name = "Axolotl Launcher";
  startupWMClass = "Axolotl Launcher";
  terminal = false;
  type = "Application";
}
