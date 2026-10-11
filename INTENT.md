# WordCraft intent

WordCraft is a clean-room, open-source, Rust-native word processor for people
who need Microsoft Word document compatibility and familiar editing workflows.
It targets native desktop platforms and the web, with fast rendering, open
formats, and command-based control for users and agents.

The upstream [roadmap](ROADMAP.md) and [architecture](docs/architecture.md)
define the product direction. Preserve document contents and recoverable errors;
never introduce proprietary assets or copy copyleft implementations.

This fork adds reproducible Nix builds and tests, native executable publication,
and a build matrix covering ARM64 macOS, ARM64/x86-64 Linux, and ARM64/x86-64
Windows. Builds cache a successful upstream fetch/rebase for 24 hours, then
automatically refresh on a clean branch. Offline builds require a fresh sync;
an explicit update bypasses the cooldown. Verification
includes the full workspace tests, existing quality gates, and inspection of
the produced binaries. Platform runtime claims require execution on that platform.

The web application is deployed through GitHub Pages with browser local storage
for documents, import from local files, and either browser storage or file export
when saving. Work remaining is tracked in [PLAN.md](PLAN.md).
