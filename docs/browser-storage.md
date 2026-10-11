# Browser document storage

In the [web editor](https://pmarreck.github.io/wordcraft/), File › Open lists documents
under **This browser**. Select one to open it, or choose **Import from your computer…**
to import a local document. Drag and drop still imports files.

**Save** (Ctrl/⌘+S) updates the open browser document. For an untitled or imported
document it first asks for a name. **Save As** offers **Save in this browser** and
**Export to your computer…**. Saving under another document's name asks before replacing
that copy. Export downloads a file, normally DOCX; it leaves the browser copy and the
document's unsaved state unchanged. Save again to update the browser copy after editing.
Opening another document with unsaved changes offers Save, Don't Save and Cancel.

Browser copies stay in this browser profile on this site's origin. They are not uploaded
to GitHub, synced between devices, or copied into the operating system's filesystem.
Clearing site data removes them. Export files to keep independent copies. AutoSave remains
unavailable in the browser; use Save explicitly.

Storage failures leave the old copy intact and the current document unsaved. If storage
is disabled or full, export your changes instead. Each encoded record is limited to
4 MiB; the browser's shared origin quota may reject smaller documents, especially when
other applications on `pmarreck.github.io` also store data. Password-protected documents
must be exported as encrypted Word files; WordCraft refuses to store them as plaintext
browser copies.

## Storage and commands

The `wordcraft.documents.v1:` localStorage namespace holds one JSON record per document,
with schema `1`, a positive revision and native document JSON. Native JSON preserves the
complete model rather than relying on an interchange-format round trip. Names contain
1–160 characters after trimming and cannot contain slashes or control characters.
Damaged or unsupported records are reported without replacing the open document.

Revision checks reject a save when another tab has already updated the stored copy;
reopen it or save with a different name. The read/check/write sequence is optimistic,
so truly simultaneous writes from separate tabs can still race. Avoid editing the same
browser document in multiple tabs. IndexedDB transactions would be a future option for
larger documents and transactional revision checks; this version uses localStorage.

All actions use the existing UI command dispatch:

| Command | Parameters | Result |
|---|---|---|
| `ui.browserList` | none | Sorted `names` |
| `ui.browserOpen` | `name` | Opened name and revision |
| `ui.browserSave` | `name`, optional positive `revision` for explicit replacement | Saved name and new revision |
| `ui.browserImport` | none | Browser file picker |
| `ui.browserExport` | optional filename `name` | Downloaded filename and byte count |

Scripted `file.save` without a path updates the bound browser document or returns an
error when there is no binding. User Save opens the naming dialog when needed.
As elsewhere in WordCraft, `run` uses the unsaved-changes guard and `execute` performs
the explicitly requested action directly.

## Verification

`./test` runs native storage tests for full-model reload, quota failures, corruption,
stale revisions, names, size limits, password handling, import/export and save prompts.
On x86-64 Linux it also runs a Chromium check against the Nix-built WASM application:
type text, save, close the browser, restart the same profile, reopen and export, then
compare the DOCX's text with the original input. WebDriver drives the painted interface;
there are no application-side test hooks or handwritten JavaScript.

GridCraft's agent compared its IndexedDB implementation with this design. The shared
rule is to clear unsaved state only after persistence succeeds; an asynchronous backend
must also account for edits made while a write is pending.
