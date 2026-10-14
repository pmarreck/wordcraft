# PLAN

## Web application

- [ ] M14.Mobile: Mirror GridCraft 4faa99e's synchronous iOS file-picker activation for document, picture and recipient imports, with bounded reads and cancellation cleanup.
- [ ] M14.Mobile: Adapt GridCraft 029c13e's phone File layout with swipeable navigation, wrapped templates and width-bounded Info/Options/Print/export controls, with overflow regressions.
- [ ] M14.Mobile: Validate software keyboards, accessibility and stylus behavior on physical iOS and Android devices.
- [ ] M14.I18n: Use the i18n skill's chosen 50 UI languages, mirror RTL layouts, provide a UI language override and label “Go back” in the most recent non-current language after each change.
- [x] M14.Ribbon: Match GridCraft's vertical-wheel ribbon browsing, preserve diagonal tails and document scroll/zoom, verify touch and five platform builds, then publish (done 2026-10-13 21:55 EDT; e624204; gh-pages 4ccce34).
- [x] M14.Mobile: Consult GridCraft and expose the full narrow-screen ribbon through independent horizontal tab/group dragging; verify touch safety, browser persistence, platform builds and publish (done 2026-10-13 17:13 EDT; bbaa772; gh-pages 273df1a).
- [x] M14.Mobile: Default phones to readable reflow with Print Layout, responsive controls and keyboard-safe viewport resizing (done 2026-10-12 16:40 EDT; b3bf5ed; gh-pages f6b88d6).
- [x] M14.Mobile: Make ordinary touch drags scroll, tap-then-drag select and two fingers pan/pinch; test cancellation and document preservation, then publish (done 2026-10-12 16:40 EDT; b3bf5ed; gh-pages f6b88d6).
- [x] M14.Fonts: Align with GridCraft's pinned, self-hosted open font catalog, on-demand browser loading, metric-compatible aliases and regression coverage; publish and verify (done 2026-10-12 13:17 EDT; 5ee3c0a; gh-pages 4335127).
- [x] M10.Web: Offer browser open/save alongside local-file import/export and preserve the unsaved-changes guard (done 2026-10-11 19:16 EDT; b1c382b).
- [x] M14.Web: Publish the browser-storage editor to GitHub Pages and verify reload/open/export behavior and CI (done 2026-10-11 19:52 EDT; 0d485df; gh-pages dbe1d12).
- [x] M10.Web: Confirm browser writes before clearing unsaved state, serialize concurrent saves, reload clean stale tabs and test conflict recovery (done 2026-10-11 19:48 EDT; 0d485df).
- [x] Explain how open-source and Google Fonts can enter the existing Rust font database and shared craft-fonts build input (done 2026-10-11 19:48 EDT; 0d485df).
- [x] M10.Web: Keep browser documents unsaved when scripted Save As starts a file download without persistence confirmation (done 2026-10-11 20:05 EDT; c7b0515).
