# PLAN log

Completed PLAN.md items retired by plan-retire, oldest retirement first.

## Retired 2026-10-11

- [x] [Build and distribution] Annotate the build entrypoints and important repository directories with dirtree notes (done 2026-10-11 12:40 EDT; 4f34b6a).
- [x] [Build and distribution] Prepare the sibling GridCraft repository with owner instructions and a handoff for its next agent (done 2026-10-11 12:40 EDT).
- [x] [Build and distribution] M14.CI: Publish the dynamic CI badge and document the exact-commit build target manifest; ./test passes and main is pushed (done 2026-10-11 15:48 EDT; dde6b25).
- [x] [Build and distribution] M14.Nix: Add pinned Nix builds, complete ./test, host ./build with safe upstream fetch/rebase, and the five-target ./build-all matrix (done 2026-10-11 13:45 EDT; 4f34b6a).
- [x] [Build and distribution] Cache successful upstream checks for 24 hours; automatically refresh stale builds and reject stale --no-update builds (done 2026-10-11 13:18 EDT; 4f34b6a).
- [x] [Build and distribution] M14.Nix: Verify all five release builds, native Linux/macOS CLI checks, debug symbols, ./test and cargo xtask ci; push main to pmarreck/wordcraft (done 2026-10-11 13:45 EDT; 4f34b6a).

## Retired 2026-10-12

- [x] [Build and distribution] M14.Build: Validate target-native packaged launchers, including cross-built ARM64 Linux; all five packages build (done 2026-10-11 17:26 EDT; 6e01091).
- [x] [Build and distribution] M14.CI: Provision the push webhook and pass all 16 CI targets for the exact pushed commit (done 2026-10-11 17:28 EDT; 6e01091).
- [x] [Build and distribution] Check available macOS signing identities, seal the packaged app after stripping, and pass native Apple signature verification (done 2026-10-11 17:26 EDT; 6e01091).
