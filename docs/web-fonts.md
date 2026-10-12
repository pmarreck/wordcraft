# Browser document fonts

The fork follows GridCraft's self-hosted font decision. The Nix web package serves
nine open font families under `fonts/`, with a GridCraft-compatible
`fonts/manifest.json`. Font binaries remain build inputs rather than source files
in this repository. Every family ships its license alongside its static TTF faces.

| Family | Document names it substitutes for |
|---|---|
| Carlito | Calibri |
| Caladea | Cambria |
| Liberation Sans | Arial, Helvetica |
| Liberation Serif | Times New Roman, Times |
| Liberation Mono | Courier New, Courier |
| Gelasio | Georgia |
| Lato | — |
| Source Sans 3 | — |
| Merriweather | — |

Regular, bold, italic and bold italic are separate files. `flake.lock` pins their
Nixpkgs sources, matching GridCraft's package selection in `nix/web-fonts.nix`.
The browser requests the small manifest at startup and offers its families in the
existing font picker. It fetches a family's faces when the active document uses
that family or an alias. Requests stay on the site's origin and work under a
GitHub Pages project prefix. There are no runtime Google Fonts requests or local
font permission prompts.

Downloaded fonts are checked before entering the Rust font database. Arrival
invalidates cached substitutes, document layout and previews without changing
document contents, font names, undo history or saved state. Failed downloads keep
the bundled fallback and log the error. A family is requested once per page load;
reloading retries failed requests. Normal HTTP caching applies; this does not
promise offline availability.

This is a deliberate fork extension to upstream's optional `CRAFT_FONTS_DIR`
script-fallback input. Large script-specific packs and fonts imported by the user
remain future work. Aptos still has no metric-compatible open substitute; the
Calibri and Cambria aliases do not resolve that separate parity gap.

Tests cover catalog parsing and path limits, alias demand, duplicate requests,
malformed font rejection, stale substitute caches, and refreshing an already open
document. The Nix site gate opens each shipped face with Fontconfig and checks
its declared family and adjacent license. Browser verification selects a served
family through the real font picker and observes its requests.
