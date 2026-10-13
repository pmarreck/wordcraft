# WordCraft architecture

> **Last reviewed:** 2026-10-13 · **Last updated:** 2026-10-13 · **Change:** minor (full mobile ribbon and scoped toolbar touch routing) · **Target:** Microsoft Word (Microsoft 365) for Mac 16.113.4

How WordCraft is built today (upstream `40c5d51`, fork packaging `6e01091`). The longer design notes in
`plan/architecture.md` are gitignored and local to the original author's machine; this file is the
committed, current summary. Rules for working in the code are in [`AGENTS.md`](../AGENTS.md).

## Layers

~85,600 lines of Rust, 15 crates, 3 apps and `xtask`. Layering is enforced by `cargo xtask layers`:
a crate may only depend on crates in lower layers, and nothing below L6 depends on egui, eframe,
winit or rfd.

| Layer | Crate | Lines | Job |
|---|---|---|---|
| L0 | `geom` | 0.2k | Units (points, twips, EMU, inches, cm), rectangles, measurement parsing |
| L1 | `doc` | 10.0k | Document model: stories, paragraphs, runs, tables, sections, styles, numbering, fields, notes, comments, revisions, equations (OMML-shaped math tree, linear/LaTeX input); edit primitives; bidi |
| L1 | `fonts` | 2.3k | Font database (bundled OFL + system fonts), metrics, outlines, shaping with HarfRust, Word font substitution (`word.rs`: Carlito ↔ Calibri, Caladea ↔ Cambria, Liberation ↔ Arial/Times/Courier), OpenType MATH |
| L1 | `proof` | 1.4k | Spelling (public-domain English word list), suggestions, rule-based grammar, hyphenation patterns |
| L2 | `layout` | 9.6k | Shaping runs, line breaking, tabs, lists, tables (row splitting, floating tables), pagination, headers/footers, notes, floating objects and wrap, math layout, fields, hit testing; produces a display list |
| L2 | `docx` | 8.4k | OOXML WordprocessingML reader and writer (Transitional and Strict namespaces, `.docx/.docm/.dotx/.dotm`), OMML, custom properties, VBA parts preserved |
| L2 | `docbin` | 4.4k | Word 97-2003 `.doc` reader ([MS-DOC]: FIB, piece table, FKPs, SPRMs, stylesheet, lists, tables, notes, pictures) over a CFB container |
| L2 | `formats` | 10.6k | TXT, Markdown, HTML, RTF, ODT and LaTeX import/export through a shared intermediate model |
| L3 | `render` | 0.6k | vello_cpu rasteriser: pages and thumbnails from the display list; PNG/JPEG |
| L3 | `pdf` | 1.6k | krilla PDF export: font subsets, selectable text, links, outline, tagging |
| L4 | `engine` | 15.2k | `Session` (document, selection, undo, layout cache, view state), **command registry** (422 `CommandSpec`s in `cmd/*.rs`), Word feature catalog (`catalog.rs`), samples, file I/O dispatch (`io.rs`), Read Aloud sentence model |
| L4 | `zotero` | 1.7k | Zotero's word-processor integration protocol answered against a `Session` |
| L4 | `control-key` | 0.1k | Per-window control-port key files shared by the app and the MCP bridge |
| L5 | `mcp` | 1.0k | MCP server: every command as a tool; headless or bridged to the running app |
| L6 | `ui-egui` | 13.1k | Word-style front end: ribbon, keytips, mini-toolbar, Backstage, canvas, rulers, panes, dialogs, Equation tab, i18n catalogs, themes, control channel, headless `ui_shot` example |
| apps | `wordcraft` | 1.6k | Desktop app (eframe + wgpu; logging, window geometry, file associations) |
| apps | `wordcraft-cli` | 0.4k | `convert`, `text`, `inspect`, `run`, `commands`, `parity`, `mcp`, `zotero` |
| apps | `wordcraft-web` | 0.3k | Browser build (eframe web runner, WebGPU with WebGL2 fallback) |

## Data flow

1. **Open:** `engine::io` picks a reader by extension (`docx`, `docbin`, `formats`, JSON) and
   produces a `doc::Document`.
2. **Edit:** every user-visible action is a command id run through `Session::run` (ribbon,
   shortcuts, command search, CLI, control channel and MCP all dispatch by id). `Session::run`
   snapshots for undo (blocks are `Arc`-shared, so snapshots are cheap) and catches an escaped
   panic, restoring the document (never-crash standard).
3. **Layout:** `layout` turns the document into pages of positioned glyphs, boxes and images
   (display list), incrementally: an edit relayouts only what changed (1.4 ms on a 188-page
   document, measured 2026-10-06).
4. **Draw:** the UI paints pages through `render` (CPU raster, cached per page, pixel-aligned) on
   an egui/wgpu canvas; PDF export walks the same display list through `pdf`.
5. **Save:** `engine::io` picks a writer by extension.

## Testing and gates

- 821 `#[test]` functions and 10 property-test blocks: model, layout, DOCX round-trip and hostile
  input, `.doc`, formats, typing parity (`engine/src/tests_typing.rs`), bidi, i18n catalogs, UI
  interaction (egui_kittest), MCP acceptance; `hostile_params_never_panic` runs every command with
  junk params.
- `cargo xtask ci`: fmt, clippy `-D warnings`, tests, asset attribution, layering, wasm build.
- `./test` runs these gates through `nix flake check --keep-going`, plus installed-app
  CLI checks, static WASM validation, build publication regressions, and real local Git
  fetch/rebase tests with an injected clock. Workspace tests include the upstream-ignored
  UI translation audit. On macOS, `./test` also verifies the installed app bundle with
  Apple's native `codesign --verify --deep --strict` through the Nix development environment.
- `cargo xtask parity` regenerates [`parity-checklist.md`](parity-checklist.md); a test keeps the
  catalog percentage above a floor that only rises.
- GitHub Actions on PRs: packaging lint, and FreeBSD / Windows ARM64 when build files change;
  release builds on pushes to `release`.

## Nix packages and build entrypoints

`flake.lock` pins Nixpkgs, Crane and rust-overlay, including the Rust toolchain.
`nix build` and `./build` select the same optimized host package: the desktop app
and CLI. `./build-all` covers ARM64 macOS/Linux/Windows and x86-64 Linux/Windows;
macOS uses a native Apple Silicon builder. Linux retains its Nix runtime closure,
macOS retains the complete `.app`, and Windows retains runtime DLLs beside its executables.
Final executable headers, including launchers, are checked against their target before
publication. Linux launchers use target-native binary wrappers. macOS bundles are sealed
with an ad hoc signature after stripping; this does not provide Developer ID signing or
notarization. Cross compilation does not establish native runtime compatibility.

`nix build .#web --out-link result-web` builds the existing browser editor with pinned
Trunk, wasm-bindgen and Binaryen, offline from locked Cargo inputs. The static output uses
project-relative asset URLs and is published from `gh-pages` to
[GitHub Pages](https://pmarreck.github.io/wordcraft/). The UI's browser document commands use an
injected storage trait; the WASM host supplies `web_sys::Storage`. Each localStorage entry
contains versioned native document JSON and is replaced with one atomic write. Saves clear
the dirty flag only after persistence succeeds. An asynchronous exclusive Web Lock guards
the synchronous read/check/write sequence; pending saves keep the document dirty and
save continuations wait for confirmation. Clean stale tabs reload on Save. File import and download export remain
available. See [browser storage](browser-storage.md) for limits and command parameters.

Native tests inject storage failures and corrupt data. On x86-64 Linux, `./test` also runs
`checks.x86_64-linux.browser`: pinned Chromium, WebDriver and OCR drive the actual editor,
restart the browser profile, reopen a saved document and check the exported DOCX's text.

The browser serves the same nine-family open font catalog as GridCraft, pinned through
Nixpkgs and copied into `fonts/` beside the app with all licenses. Its manifest populates
the existing picker; the Rust web shell fetches static TTF faces only when the document
uses a family or a metric-compatible alias. Arrival refreshes cached font substitutes,
layout and previews without editing or marking the document unsaved. Font binaries stay
outside this repository and the WASM binary. The separate optional `CRAFT_FONTS_DIR`
input still supplies script fallback faces; its WASM filter selects BIZ UDPGothic Regular.
See [browser fonts](web-fonts.md) for the catalog, limits and deliberate upstream divergence.

The compact UI reuses the full desktop ribbon, command registry and canvas. Its tab strip
and command groups scroll horizontally and independently. The scoped ribbon touch adapter
classifies taps before activating controls and converts swipes into horizontal wheel input,
including swipes over text fields. A bounded, timestamped
gesture state machine separates finger scrolling, tap-then-drag selection and midpoint
pinch/pan from mouse input. The web adapter filters eframe's duplicate pointer callbacks
over the document and ribbon while preserving browser user activation for UI actions. VisualViewport
events resize the canvas around software keyboards without taking over browser page zoom.
Phone reflow changes view state only; exported pages keep the document's layout settings.
See [mobile editing](mobile.md) for the gesture contract and regression coverage.

Both scripts check Git metadata for the last successful upstream fetch/rebase.
A cached check is valid for less than 24 hours and only while HEAD contains the
recorded upstream commit. Missing or expired checks automatically refresh on a
clean branch. `--update` forces refresh, and `--no-update` requires a fresh cache.
Conflicts, failed fetches and concurrent updates stop before compilation.
Successful products are selected atomically under `bin/<os>/<arch>/` from
retained `.nix-out/<rust-target>/<profile>/` products.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-13 | minor | Full mobile ribbon with independent scroll areas and scoped pre-egui touch routing, following GridCraft guidance |
| 2026-10-12 | minor | Responsive shell over existing commands, pure touch state, browser pointer ordering and visible viewport adapters |
| 2026-10-12 | minor | GridCraft-compatible self-hosted font catalog, lazy face loading and cache refresh |
| 2026-10-11 | minor | Web Locks, pending-save continuations and clean stale-tab reload; documented Rust font-loading extension path |
| 2026-10-11 | minor | Browser document commands, injected localStorage adapter, lossless persistence and real-browser regression check |
| 2026-10-11 | minor | Nix-built WASM site on GitHub Pages, target-native launcher checks and macOS bundle sealing and verification |
| 2026-10-11 | minor | Pinned Nix build matrix, full checks, platform products and daily upstream sync |
| 2026-10-10 | major | First committed architecture doc, from the crates' sources and `AGENTS.md` |
