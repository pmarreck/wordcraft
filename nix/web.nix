{
  pkgs,
  crane,
  src,
}:
let
  toolchain = pkgs.rust-bin.stable.latest.default.override {
    targets = [ "wasm32-unknown-unknown" ];
  };
  craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
  args = {
    pname = "wordcraft-web";
    version = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).workspace.package.version;
    inherit src;
    strictDeps = true;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
    cargoExtraArgs = "--locked -p wordcraft-web --target wasm32-unknown-unknown";
    doCheck = false;
    WORDCRAFT_BUILD_SHA = "nix";
    WORDCRAFT_BUILD_DATE = "reproducible";
  };
  app = craneLib.mkCargoDerivation (
    args
    // {
      cargoArtifacts = craneLib.buildDepsOnly args;
      nativeBuildInputs = [
        pkgs.trunk
        pkgs.wasm-bindgen-cli
        pkgs.binaryen
        pkgs.wasm-tools
      ];
      buildPhaseCargoCommand = ''
        cd apps/wordcraft-web
        trunk build --release --offline --frozen --dist "$out"
      '';
      installPhaseCommand = ''
        touch "$out/.nojekyll"
        cp "$src/LICENSE-MIT" "$src/LICENSE-APACHE" "$src/NOTICE" "$src/ATTRIBUTION.md" "$out/"
        mkdir -p "$out/assets/fonts"
        cp "$src"/assets/fonts/OFL-*.txt "$out/assets/fonts/"
      '';
      doInstallCargoArtifacts = false;
    }
  );
in
pkgs.runCommand "wordcraft-web-site-${args.version}"
  {
    nativeBuildInputs = [
      pkgs.jq
      pkgs.fontconfig
      pkgs.wasm-tools
    ];
    FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = [ ]; };
  }
  ''
    cp -r ${app} "$out"
    chmod u+w "$out"
    cp -r ${import ./web-fonts.nix { inherit pkgs; }} "$out/fonts"
    export XDG_CACHE_HOME="$TMPDIR/font-cache"
    mkdir -p "$XDG_CACHE_HOME"
    bash ${../tests/cli/web-assets} "$out"
  ''
