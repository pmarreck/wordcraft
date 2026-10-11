{
  pkgs,
  src,
  web,
}:
let
  fonts = pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; };
  dependencies = with pkgs; [
    bash
    chromium
    chromedriver
    darkhttpd
    curl
    jq
    tesseract
    unzip
  ];
in
{
  shell = pkgs.mkShell {
    packages = dependencies;
    FONTCONFIG_FILE = fonts;
  };
  check =
    pkgs.runCommand "wordcraft-browser-storage-tests"
      {
        nativeBuildInputs = dependencies;
        FONTCONFIG_FILE = fonts;
      }
      ''
        bash ${src}/tests/cli/browser-storage ${web}
        touch "$out"
      '';
}
