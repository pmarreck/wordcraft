{
  description = "WordCraft desktop and CLI: reproducible builds, tests, and cross-platform packages";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      rust-overlay,
    }:
    let
      lib = nixpkgs.lib;
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forSystems = lib.genAttrs systems;
      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
            (import ./nix/windows-toolchain.nix)
          ];
        };
      # Keep non-Rust inputs (fonts, icons, fixtures and contributor data).
      # Never capture local products or the external agent-instructions symlink.
      src = lib.cleanSourceWith {
        src = self;
        filter =
          path: type:
          let
            name = baseNameOf path;
          in
          lib.cleanSourceFilter path type
          && !(builtins.elem name [
            "bin"
            "target"
            "dist"
            "plan"
            ".nix-out"
            "result"
            "AGENTS.md"
            "AGENTS_original.md"
            ".dirtree-state"
            "PLAN.md"
            "INTENT.md"
            "ROADMAP.md"
            "README.md"
            "flake.nix"
            "flake.lock"
            "nix"
          ])
          && !(lib.hasPrefix "result-" name || lib.hasPrefix "target-" name);
      };
      # Shell helpers and progress documents are separate check inputs. Editing
      # them must not trigger a new optimized build of the entire Rust GUI.
      rustSrc = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./.cargo
          ./clippy.toml
          ./rustfmt.toml
          ./crates
          ./apps
          ./xtask
          ./assets
          ./contributors
          ./packaging
          ./ATTRIBUTION.md
          ./LICENSE-MIT
          ./LICENSE-APACHE
          ./NOTICE
        ];
      };
      mk =
        buildSystem: targetSystem: crossSystem: profile:
        let
          buildPkgs = pkgsFor buildSystem;
          pkgs =
            if crossSystem == null then
              pkgsFor targetSystem
            else
              import nixpkgs {
                localSystem = buildSystem;
                # Build-only cross packages never need Wine/QEMU. In particular,
                # evaluating Wine on Darwin is unsupported by Nixpkgs.
                crossSystem = crossSystem // {
                  emulatorAvailable = _: false;
                };
                overlays = [
                  rust-overlay.overlays.default
                  (import ./nix/windows-toolchain.nix)
                ];
              };
        in
        import ./nix/package.nix {
          inherit
            pkgs
            buildPkgs
            crane
            profile
            ;
          src = rustSrc;
        };
      native = system: mk system system null "release";
      browser =
        system:
        import ./nix/browser-test.nix {
          pkgs = pkgsFor system;
          inherit src;
          web = self.packages.${system}.web;
        };
    in
    {
      packages = forSystems (
        system:
        let
          targets = {
            # Apple frameworks require a native Darwin builder. Nix can use
            # a configured remote builder when invoked from Linux.
            aarch64-macos = profile: mk "aarch64-darwin" "aarch64-darwin" null profile;
            aarch64-linux =
              profile:
              mk system "aarch64-linux" (
                if system == "aarch64-linux" then null else lib.systems.examples.aarch64-multiplatform
              ) profile;
            x86_64-linux =
              profile:
              mk system "x86_64-linux" (
                if system == "x86_64-linux" then null else lib.systems.examples.gnu64
              ) profile;
            aarch64-windows = profile: mk system system lib.systems.examples.mingw-ucrt-aarch64 profile;
            x86_64-windows = profile: mk system system lib.systems.examples.mingwW64 profile;
          };
        in
        lib.mapAttrs (_: make: (make "release").package) targets
        // lib.mapAttrs' (name: make: lib.nameValuePair "${name}-debug" (make "dev").package) targets
        // {
          default = (native system).package;
          web = import ./nix/web.nix {
            pkgs = pkgsFor system;
            inherit crane;
            src = rustSrc;
          };
        }
      );

      checks = forSystems (
        system:
        let
          pkgs = pkgsFor system;
          n = native system;
          dev = mk system system null "dev";
          common = dev.args // {
            cargoArtifacts = dev.cargoArtifacts;
          };
        in
        {
          build = n.package;
          web = self.packages.${system}.web;
          test = dev.craneLib.cargoTest (
            common
            // {
              MUTE_DEBUG_STATUS = "1";
              cargoTestExtraArgs = "--no-fail-fast -- --include-ignored";
            }
          );
          clippy = dev.craneLib.cargoClippy (
            common // { cargoClippyExtraArgs = "--all-targets -- -D warnings"; }
          );
          fmt = dev.craneLib.cargoFmt {
            src = rustSrc;
            pname = "wordcraft";
          };
          sources = pkgs.runCommand "wordcraft-source-classification" { } ''
            # Classify sets of real inputs: code/resources must survive both
            # filters, local agent files/products must survive neither.
            for tree in ${src} ${rustSrc}; do
              for path in Cargo.toml Cargo.lock .cargo/config.toml crates/doc/src/lib.rs \
                apps/wordcraft/src/main.rs apps/wordcraft-web/src/browser_storage.rs \
                crates/ui-egui/src/browser_documents.rs assets/fonts/Inter-SemiBold.ttf \
                assets/app-icon/wordcraft.icns packaging/macos/Info.plist.in ATTRIBUTION.md; do
                test -f "$tree/$path"
              done
              for path in AGENTS.md AGENTS_original.md .dirtree-state PLAN.md INTENT.md \
                README.md flake.nix flake.lock nix bin target .nix-out result; do
                test ! -e "$tree/$path" && test ! -L "$tree/$path"
              done
            done
            for path in build build-all test scripts/build-common.sh tests/cli/upstream-sync tests/cli/package-headers tests/cli/web-assets tests/cli/macos-signature tests/cli/browser-storage tests/cli/browser-mobile; do
              test -f "${src}/$path"
              test ! -e "${rustSrc}/$path"
            done
            touch "$out"
          '';
          scripts =
            pkgs.runCommand "wordcraft-build-script-tests"
              {
                nativeBuildInputs = [
                  pkgs.bash
                  pkgs.git
                  pkgs.coreutils
                  pkgs.gawk
                  pkgs.shellcheck
                ];
              }
              ''
                cp -r ${src} source
                chmod -R u+w source
                cd source
                export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
                patchShebangs build build-all test
                failures=0
                shellcheck -x build build-all test scripts/build-common.sh tests/cli/build-scripts tests/cli/upstream-sync tests/cli/apps tests/cli/package-headers tests/cli/web-assets tests/cli/macos-signature tests/cli/browser-storage tests/cli/browser-mobile || failures=$((failures + 1))
                for suite in tests/cli/build-scripts tests/cli/upstream-sync; do
                  bash "$suite" || failures=$((failures + 1))
                done
                test "$failures" -eq 0
                touch "$out"
              '';
          cli =
            pkgs.runCommand "wordcraft-cli-tests"
              {
                nativeBuildInputs = [
                  pkgs.bash
                  pkgs.coreutils
                  pkgs.gnugrep
                ];
              }
              ''
                bash ${src}/tests/cli/apps ${n.package}/bin
                touch "$out"
              '';
        }
        // lib.optionalAttrs (system == "x86_64-linux") { browser = (browser system).check; }
        // lib.genAttrs [ "assets" "layers" "wasm" ] (
          step:
          dev.craneLib.mkCargoDerivation (
            common
            // {
              pname = "wordcraft-${step}";
              nativeBuildInputs = dev.args.nativeBuildInputs ++ lib.optional (step == "assets") pkgs.git;
              src = if step == "assets" then src else rustSrc;
              # The sandbox has no .git directory. Give the existing attribution
              # gate a real index so it checks the source instead of an empty set.
              buildPhaseCargoCommand =
                lib.optionalString (step == "assets") ''
                  git -c init.defaultBranch=main init -q
                  git add -- .
                ''
                + "cargo xtask ${step}";
              installPhaseCommand = ''mkdir -p "$out"; touch "$out/passed"'';
              doInstallCargoArtifacts = false;
              doCheck = false;
            }
          )
        )
      );

      devShells = forSystems (
        system:
        let
          pkgs = pkgsFor system;
          n = native system;
        in
        {
          default = n.craneLib.devShell {
            packages = [
              pkgs.pkg-config
              pkgs.shellcheck
              pkgs.git
            ];
            inputsFrom = [ n.package ];
          };
        }
        // lib.optionalAttrs (system == "x86_64-linux") { browser = (browser system).shell; }
      );
      formatter = forSystems (system: (pkgsFor system).nixfmt-tree);
    };
}
