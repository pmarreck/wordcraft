# PLAN

## Build and distribution

- [ ] M14.CI: Configure the Mechatron Prime push webhook, publish its dynamic badge and verify CI for the pushed commit.
- [x] M14.Nix: Add pinned Nix builds, complete ./test, host ./build with safe upstream fetch/rebase, and the five-target ./build-all matrix (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] Cache successful upstream checks for 24 hours; automatically refresh stale builds and reject stale --no-update builds (done 2026-10-11 13:18 EDT; 4f34b6a).
- [x] M14.Nix: Verify all five release builds, native Linux/macOS CLI checks, debug symbols, ./test and cargo xtask ci; push main to pmarreck/wordcraft (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] Annotate the build entrypoints and important repository directories with dirtree notes (done 2026-10-11 12:40 EDT; 4f34b6a).
- [x] Prepare the sibling GridCraft repository with owner instructions and a handoff for its next agent (done 2026-10-11 12:40 EDT).

## Web application

- [ ] Compile WordCraft to web/WASM and deploy on GitHub Pages with browser local storage as its filesystem, local-file import, and save choices for browser storage or export to the local filesystem.
