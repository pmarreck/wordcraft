# PLAN

## Build and distribution

- [ ] M14.Build: Fix target architecture validation for packaged launchers, including cross-built ARM64 Linux.
- [ ] M14.CI: Configure the Mechatron Prime push webhook and verify CI for the pushed commit.
- [ ] Check signing identities on the available macOS host and verify the packaged application's signature.
- [x] M14.CI: Publish the dynamic CI badge and document the exact-commit build target manifest; ./test passes and main is pushed (done 2026-10-11 15:48 EDT; dde6b25).
- [x] M14.Nix: Add pinned Nix builds, complete ./test, host ./build with safe upstream fetch/rebase, and the five-target ./build-all matrix (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] Cache successful upstream checks for 24 hours; automatically refresh stale builds and reject stale --no-update builds (done 2026-10-11 13:18 EDT; 4f34b6a).
- [x] M14.Nix: Verify all five release builds, native Linux/macOS CLI checks, debug symbols, ./test and cargo xtask ci; push main to pmarreck/wordcraft (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] Annotate the build entrypoints and important repository directories with dirtree notes (done 2026-10-11 12:40 EDT; 4f34b6a).
- [x] Prepare the sibling GridCraft repository with owner instructions and a handoff for its next agent (done 2026-10-11 12:40 EDT).

## Web application

- [ ] M14.Web: Build and deploy the existing WASM editor through Nix to GitHub Pages and verify the live /wordcraft/ URL.
- [ ] Compile WordCraft to web/WASM and deploy on GitHub Pages with browser local storage as its filesystem, local-file import, and save choices for browser storage or export to the local filesystem.
