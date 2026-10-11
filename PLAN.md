# PLAN

## Build and distribution

- [x] M14.Build: Validate target-native packaged launchers, including cross-built ARM64 Linux; all five packages build (done 2026-10-11 17:26 EDT; 6e01091).
- [x] M14.CI: Provision the push webhook and pass all 16 CI targets for the exact pushed commit (done 2026-10-11 17:28 EDT; 6e01091).
- [x] Check available macOS signing identities, seal the packaged app after stripping, and pass native Apple signature verification (done 2026-10-11 17:26 EDT; 6e01091).

## Web application

- [x] M14.Web: Build and deploy the WASM editor through Nix to GitHub Pages; verify live rendering and served WASM bytes (done 2026-10-11 17:28 EDT; 6e01091; gh-pages 6b03553).
- [x] M10.Web: Add lossless browser document storage with bounded inputs, failure recovery and persistence regression tests (done 2026-10-11 19:16 EDT; b1c382b).
- [x] M10.Web: Offer browser open/save alongside local-file import/export and preserve the unsaved-changes guard (done 2026-10-11 19:16 EDT; b1c382b).
- [x] M14.Web: Publish the browser-storage editor to GitHub Pages and verify reload/open/export behavior and CI (done 2026-10-11 19:52 EDT; 0d485df; gh-pages dbe1d12).
- [x] M10.Web: Confirm browser writes before clearing unsaved state, serialize concurrent saves, reload clean stale tabs and test conflict recovery (done 2026-10-11 19:48 EDT; 0d485df).
- [x] Explain how open-source and Google Fonts can enter the existing Rust font database and shared craft-fonts build input (done 2026-10-11 19:48 EDT; 0d485df).
