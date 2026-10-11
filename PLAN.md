# PLAN

## Build and distribution

- [ ] M14.Nix: Add pinned Nix builds, complete ./test, host ./build with safe upstream fetch/rebase, and the five-target ./build-all matrix.
- [x] Cache successful upstream checks for 24 hours; automatically refresh stale builds and reject stale --no-update builds (done 2026-10-11 13:18 EDT).
- [ ] Enable the prepared Mechatron Prime push webhook if approved and verify CI for the pushed commit.
- [ ] M14.Nix: Verify build artifacts and all quality gates, then push main to the contributor fork.
- [x] Annotate the build entrypoints and important repository directories with dirtree notes (done 2026-10-11 12:40 EDT).
- [x] Prepare the sibling GridCraft repository with owner instructions and a handoff for its next agent (done 2026-10-11 12:40 EDT).

## Web application

- [ ] Compile WordCraft to web/WASM and deploy on GitHub Pages with browser local storage as its filesystem, local-file import, and save choices for browser storage or export to the local filesystem.
