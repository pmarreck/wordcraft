# PLAN

## Web application

- [ ] M14.Mobile: Ask GridCraft for narrow-toolbar guidance, then expose ribbon commands through horizontal drag scrolling and browsable tabs without requiring search.
- [ ] M14.Mobile: Validate software keyboards, accessibility and stylus behavior on physical iOS and Android devices.
- [x] M14.Mobile: Default phones to readable reflow with Print Layout, responsive controls and keyboard-safe viewport resizing (done 2026-10-12 16:40 EDT; b3bf5ed; gh-pages f6b88d6).
- [x] M14.Mobile: Make ordinary touch drags scroll, tap-then-drag select and two fingers pan/pinch; test cancellation and document preservation, then publish (done 2026-10-12 16:40 EDT; b3bf5ed; gh-pages f6b88d6).
- [x] M14.Fonts: Align with GridCraft's pinned, self-hosted open font catalog, on-demand browser loading, metric-compatible aliases and regression coverage; publish and verify (done 2026-10-12 13:17 EDT; 5ee3c0a; gh-pages 4335127).
- [x] M14.Web: Build and deploy the WASM editor through Nix to GitHub Pages; verify live rendering and served WASM bytes (done 2026-10-11 17:28 EDT; 6e01091; gh-pages 6b03553).
- [x] M10.Web: Add lossless browser document storage with bounded inputs, failure recovery and persistence regression tests (done 2026-10-11 19:16 EDT; b1c382b).
- [x] M10.Web: Offer browser open/save alongside local-file import/export and preserve the unsaved-changes guard (done 2026-10-11 19:16 EDT; b1c382b).
- [x] M14.Web: Publish the browser-storage editor to GitHub Pages and verify reload/open/export behavior and CI (done 2026-10-11 19:52 EDT; 0d485df; gh-pages dbe1d12).
- [x] M10.Web: Confirm browser writes before clearing unsaved state, serialize concurrent saves, reload clean stale tabs and test conflict recovery (done 2026-10-11 19:48 EDT; 0d485df).
- [x] Explain how open-source and Google Fonts can enter the existing Rust font database and shared craft-fonts build input (done 2026-10-11 19:48 EDT; 0d485df).
- [x] M10.Web: Keep browser documents unsaved when scripted Save As starts a file download without persistence confirmation (done 2026-10-11 20:05 EDT; c7b0515).
