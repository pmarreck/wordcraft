# PLAN

## Web application

- [ ] CI.Fork: Audit migrated legacy workflow coverage in Mechatron Prime, including FreeBSD/native Windows, fonts, packaging lint, artifacts and scheduling; record evidence and close scoped gaps.
- [ ] M14.Mobile: Verify clipped dialog text fields retain IME focus; adapt GridCraft 17b7427 with offscreen-input regressions where needed.
- [ ] M14.Mobile: Mirror GridCraft 4faa99e/d8ba3b0's synchronous iOS file picker for document, picture and recipient imports, with bounded fallible allocation and cancellation cleanup.
- [ ] M14.Mobile: Adapt GridCraft 029c13e's phone File layout with swipeable navigation, wrapped templates and width-bounded Info/Options/Print/export controls, with overflow regressions.
- [ ] M14.Mobile: Validate software keyboards, accessibility and stylus behavior on physical iOS and Android devices.
- [ ] M14.Ribbon: Align accessible control names and touch-sized 44-point controls with GridCraft 0f1863a/880fe91, preserving title-bar sizing and mouse behavior.
- [ ] M14.I18n: Use the i18n skill's chosen 50 UI languages, mirror RTL layouts, provide a UI language override and label “Go back” in the most recent non-current language after each change.
- [x] M14.Mobile: Retain clipped comment/reply input and drafts through scrolling, commit once on blur and restore with one undo; full tests, five builds and Apple bundle verification pass (done 2026-10-14 16:33 EDT).
- [x] M14.Mobile: Keep document scrolling drags from opening the keyboard; only completed taps move the caret and request text input, with native and browser regressions (done 2026-10-14 11:41 EDT; 7070f4d).
- [x] M14.Ribbon: Match GridCraft's vertical-wheel ribbon browsing, preserve diagonal tails and document scroll/zoom, verify touch and five platform builds, then publish (done 2026-10-13 21:55 EDT; e624204; gh-pages 4ccce34).
- [x] M14.Mobile: Consult GridCraft and expose the full narrow-screen ribbon through independent horizontal tab/group dragging; verify touch safety, browser persistence, platform builds and publish (done 2026-10-13 17:13 EDT; bbaa772; gh-pages 273df1a).
- [x] M14.Mobile: Default phones to readable reflow with Print Layout, responsive controls and keyboard-safe viewport resizing (done 2026-10-12 16:40 EDT; b3bf5ed; gh-pages f6b88d6).
- [x] M14.Mobile: Make ordinary touch drags scroll, tap-then-drag select and two fingers pan/pinch; test cancellation and document preservation, then publish (done 2026-10-12 16:40 EDT; b3bf5ed; gh-pages f6b88d6).
- [x] M14.Fonts: Align with GridCraft's pinned, self-hosted open font catalog, on-demand browser loading, metric-compatible aliases and regression coverage; publish and verify (done 2026-10-12 13:17 EDT; 5ee3c0a; gh-pages 4335127).
- [x] M14.Web: Publish the browser-storage editor to GitHub Pages and verify reload/open/export behavior and CI (done 2026-10-11 19:52 EDT; 0d485df; gh-pages dbe1d12).
- [x] Explain how open-source and Google Fonts can enter the existing Rust font database and shared craft-fonts build input (done 2026-10-11 19:48 EDT; 0d485df).
- [x] M10.Web: Keep browser documents unsaved when scripted Save As starts a file download without persistence confirmation (done 2026-10-11 20:05 EDT; c7b0515).
