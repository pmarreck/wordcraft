{
  pkgs,
  buildPkgs,
  crane,
  src,
  profile,
}:
let
  lib = pkgs.lib;
  platform = pkgs.stdenv.hostPlatform;
  target = platform.rust.rustcTarget;
  makeToolchain =
    p:
    p.rust-bin.stable.latest.default.override {
      extensions = [
        "rust-src"
        "rustfmt"
        "clippy"
      ];
      targets = [
        target
        "wasm32-unknown-unknown"
      ];
    };
  toolchain = makeToolchain buildPkgs;
  craneLib = (crane.mkLib pkgs).overrideToolchain makeToolchain;
  version = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).workspace.package.version;
  args = {
    pname = "wordcraft";
    inherit src version;
    strictDeps = true;
    CARGO_PROFILE = profile;
    cargoExtraArgs = "--workspace --locked";
    # The release graph installs the two user-facing executables; xtask and
    # the native web stub remain test/build tools rather than installed apps.
    nativeBuildInputs = [
      buildPkgs.pkg-config
      buildPkgs.makeWrapper
      buildPkgs.file
      buildPkgs.gnugrep
    ];
    buildInputs = lib.optionals platform.isLinux [
      pkgs.wayland
      pkgs.libxkbcommon
    ];
    WORDCRAFT_BUILD_SHA = "nix";
    WORDCRAFT_BUILD_DATE = "reproducible";
  };
  cargoArtifacts = craneLib.buildDepsOnly (args // { doCheck = profile == "dev"; });
  package = craneLib.buildPackage (
    args
    // {
      inherit cargoArtifacts;
      # A cross-built launcher needs the target compiler, not the build host's Bash.
      nativeBuildInputs =
        lib.remove buildPkgs.makeWrapper args.nativeBuildInputs
        ++ lib.optionals (platform.isLinux || platform.isDarwin) [ pkgs.buildPackages.makeBinaryWrapper ]
        ++ lib.optionals platform.isDarwin [ buildPkgs.rcodesign ];
      cargoExtraArgs = "--locked -p wordcraft -p wordcraft-cli";
      doCheck = false;
      postInstall = ''
        mkdir -p "$out/share/doc/wordcraft/fonts"
        cp LICENSE-MIT LICENSE-APACHE NOTICE ATTRIBUTION.md "$out/share/doc/wordcraft/"
        cp assets/fonts/*.txt "$out/share/doc/wordcraft/fonts/"
      ''
      + lib.optionalString platform.isLinux ''
        wrapProgram "$out/bin/wordcraft" --prefix LD_LIBRARY_PATH : ${
          lib.makeLibraryPath [
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.libGL
            pkgs.vulkan-loader
            pkgs.libx11
            pkgs.libxcursor
            pkgs.libxi
            pkgs.libxrandr
            pkgs.libxcb
          ]
        }
      ''
      + lib.optionalString platform.isDarwin ''
        app="$out/Applications/WordCraft.app/Contents"
        mkdir -p "$app/MacOS" "$app/Resources"
        mv "$out/bin/wordcraft" "$app/MacOS/WordCraft"
        cp assets/app-icon/wordcraft.icns "$app/Resources/WordCraft.icns"
        substitute packaging/macos/Info.plist.in "$app/Info.plist" \
          --replace-fail '@VERSION@' '${version}' \
          --replace-fail '@SHORT_VERSION@' '${version}' \
          --replace-fail '@BUILD_SHA@' 'nix' \
          --replace-fail '<string>11.0</string>' '<string>${platform.darwinMinVersion}</string>'
        printf 'APPL????' > "$app/PkgInfo"
        makeWrapper "$app/MacOS/WordCraft" "$out/bin/wordcraft"
      ''
      + lib.optionalString platform.isWindows ''
        # Keep runtime DLLs beside the executables for native Windows loading.
        for dll in ${pkgs.stdenv.cc.cc}/bin/*.dll ${
          pkgs.stdenv.cc.cc.lib or pkgs.stdenv.cc.cc
        }/bin/*.dll ${pkgs.windows.pthreads}/bin/*.dll; do
          if [ -f "$dll" ]; then cp -L "$dll" "$out/bin/"; fi
        done
      '';
      postFixup = ''
        bash ${../tests/cli/package-headers} "$out" '${target}'
      ''
      + lib.optionalString platform.isWindows ''
        # Nix's DLL hook may add store-relative links. Materialize them for export.
        for dll in "$out/bin/"*.dll; do
          if [ -L "$dll" ]; then
            cp -L "$dll" "$dll.copy"
            mv -f "$dll.copy" "$dll"
          fi
          if [ -e "$dll" ]; then test -f "$dll" && test ! -L "$dll"; fi
        done
      ''
      + lib.optionalString platform.isDarwin ''
        # Seal the complete bundle after stripping; linker signatures only cover the executable.
        rcodesign sign "$out/Applications/WordCraft.app"
        test -s "$out/Applications/WordCraft.app/Contents/_CodeSignature/CodeResources"
      '';
      meta = {
        description = "Open-source word processor with DOCX support and a headless CLI";
        homepage = "https://github.com/storytold/wordcraft";
        license = [
          lib.licenses.mit
          lib.licenses.asl20
        ];
        mainProgram = "wordcraft";
      };
    }
    // lib.optionalAttrs (profile == "dev") {
      dontStrip = true;
    }
  );
in
{
  inherit
    package
    craneLib
    args
    cargoArtifacts
    toolchain
    ;
}
