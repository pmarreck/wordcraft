# PLAN

## Build and distribution

- [x] M14.Build: Validate target-native packaged launchers, including cross-built ARM64 Linux; all five packages build (done 2026-10-11 17:26 EDT; 6e01091).
- [x] M14.CI: Provision the push webhook and pass all 16 CI targets for the exact pushed commit (done 2026-10-11 17:28 EDT; 6e01091).
- [x] Check available macOS signing identities, seal the packaged app after stripping, and pass native Apple signature verification (done 2026-10-11 17:26 EDT; 6e01091).
- [x] M14.CI: Publish the dynamic CI badge and document the exact-commit build target manifest; ./test passes and main is pushed (done 2026-10-11 15:48 EDT; dde6b25).
- [x] M14.Nix: Add pinned Nix builds, complete ./test, host ./build with safe upstream fetch/rebase, and the five-target ./build-all matrix (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] Cache successful upstream checks for 24 hours; automatically refresh stale builds and reject stale --no-update builds (done 2026-10-11 13:18 EDT; 4f34b6a).
- [x] M14.Nix: Verify all five release builds, native Linux/macOS CLI checks, debug symbols, ./test and cargo xtask ci; push main to pmarreck/wordcraft (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] Annotate the build entrypoints and important repository directories with dirtree notes (done 2026-10-11 12:40 EDT; 4f34b6a).
- [x] Prepare the sibling GridCraft repository with owner instructions and a handoff for its next agent (done 2026-10-11 12:40 EDT).

## Web application

- [x] M14.Web: Build and deploy the WASM editor through Nix to GitHub Pages; verify live rendering and served WASM bytes (done 2026-10-11 17:28 EDT; 6e01091; gh-pages 6b03553).
- [ ] Add a browser document filesystem with local-storage open/save choices alongside the existing local-file import and export.
