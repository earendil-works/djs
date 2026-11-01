# 0114: Align the guide's design with Elanor (Earendil's design system), using its published packages

status: done
agent: design
branch: issue/0114-docs-elanor
depends-on: 0103
guide: all pages (visual only; no content changes)
touches: crates/djs-docs/ (templates, assets, build: the site's look and asset pipeline), docs/guide/img/*.svg (only their style blocks), .github/workflows/docs.yml, a package.json for the docs assets if needed. Not docs/guide/*.md content (the docs agent owns it, 0113).

## Goal
The owner wants the guide (djs-docs output, live at earendil-works.github.io/djs) to look
like Earendil's design system, **Elanor**: https://github.com/earendil-works/elanor
(public repo; a read-only clone is fine). Ideally use the **published** Elanor packages,
not a copy.

What a first look found (coordinator, 2026-11-01):
- Packages: design tokens (`@earendil-works/design-tokens`, CSS exports per brand:
  `elanor.css`, `earendil.css`, `pi.css`, …, built with Terrazzo), fonts
  (`@earendil-works/elanor-fonts`: Earendil, Commit Mono, Mallos Serif, Departure Mono),
  icons (SVG and React, plain or "boiled" hand-drawn), logos, a Shiki theme
  (`packages/shiki-theme`, useful for code highlighting), React components (Base UI +
  CSS Modules), and a stylelint plugin for token usage.
- Its docs app is Astro (`apps/docs`): look at it for the intended typography, layout,
  navigation, and code styling.
- Packages are **not on public npm**: every push to elanor's main publishes preview builds
  to https://pkg.earendil.com, which needs an Earendil Works login
  (`npm login --registry=https://pkg.earendil.com`), and builds expire after 30 days
  unless pinned to a full commit SHA. So "using the published version" needs auth
  locally and in the Pages workflow (a token as a GitHub secret, which the owner would
  set up), plus a pinning strategy.

## Deliverables
- [ ] Research first, report to the coordinator before building: which packages to use
      (tokens CSS, fonts, icons, the Shiki theme; React is likely not needed for a static
      generator: confirm), how djs-docs consumes them (an npm install of the published
      packages at build time, pinned to a commit SHA, vs vendoring generated CSS/fonts
      into the repo, with the auth and expiry implications), which brand theme (elanor?
      earendil?), and what the owner must provide (a pkg.earendil.com token for CI?).
- [ ] After approval: djs-docs' templates, CSS and SVG diagram styles aligned with Elanor
      (typography, colors, spacing, light/dark, code highlighting with the Elanor Shiki
      theme if feasible in a Rust generator, or pre-highlighted at build time), the Pages
      workflow updated, `djs-docs check` and build green, and screenshots (headless
      Chrome) before/after.

## Notes
- The docs agent (0113) is adding search, generated status numbers and live disassembly
  in djs-docs at the same time: coordinate template/asset changes through the
  coordinator. This issue owns the look; 0113 owns content and features.
- Don't install anything globally; a local package.json + node_modules (gitignored) is
  fine. Node ≥24 is available.

## Log
- 2026-11-01 (design): research. Cloned elanor@ccdb0d9 and earendil-works/pkg (README)
  into work/scratch/0114; installed the published packages with the existing npm login.
  Findings: earendil-works workflows get install tokens through GitHub OIDC (no secret),
  but builds expire 30 days after upload even when pinned; elanor-react is CSS Modules
  (hashed classes) and unusable from a static generator; the shiki theme is a
  scope -> token map; brands elanor/earendil/pi/radius differ only in the canvas color.
  Proposal sent; the coordinator approved vendoring, the earendil brand, the Rust
  highlighter on the theme's roles, the Earendil mark in the header.
- Built: `scripts/elanor-update.sh [SHA|main] [BRAND]` + `crates/djs-docs/elanor/`
  (package.json and lockfile pinned to elanor ccdb0d9e) -> `crates/djs-docs/assets/elanor/`
  (tokens.css, colors.tsv, fonts + OFL licenses, 5 icons + Apache license, Earendil
  logos + favicon + brand guidelines, VERSION). The script is reproducible: a rerun at the
  pinned SHA leaves the copy unchanged.
- style.css rewritten against the tokens only (unit test: every var() exists in
  tokens.css); page.js: light/dark toggle (data-color-scheme + localStorage, set before
  first paint in <head>), chapter menu under 61.625rem, TOC current section. Template:
  Earendil mark + "djs" identity, search with icon, Source, toggle; footer with the
  licenses. Markdown tables get a `.table-scroll` wrapper in site.rs (no render.rs change).
- highlight.rs: the Shiki theme's eight roles; svg.rs: one stylesheet for generated and
  hand-drawn diagrams, scoped per diagram (`svg.dg-NAME`, ids `NAME-a`/`NAME-hatch`),
  colors `var(--semantic-color-…, #fallback)` from colors.tsv; regen now rewrites the
  `<style>`/`<defs>` of docs/guide/img/*.svg too (main.rs load), so `check` catches stale ones.
- Owner request: nothing may stick out of its column. Cause: `code { white-space: nowrap }`
  in the old CSS (ref-intrinsics overflowed by 85 px at 1440 px, more on smaller screens).
  Now inline code wraps (`overflow-wrap: anywhere`, except names in a table's first
  column), code blocks and wide tables scroll inside their own box. New
  `scripts/docs-overflow.sh` (headless Chrome, crates/djs-docs/tools/overflow.html)
  checks every page at 1440/1000/390 px: no overflow at any width; it reports the old
  CSS's overflow when pointed at it. Not in the gate (needs Chrome).
- Screenshots (work/scratch/0114): before-values-light.png, before-ops-light.png,
  djs-before.png (dark) vs after-values-light.png, after-values-dark.png, a-compiler.png,
  b-gc.png, d-ops.png, a-intro-dark.png, f-live.png (live example run in djs-wasm),
  e-search.png, b-ops-m.png and a-mobile.png (390 px).
- Guide pages: docs/guide/README.md (site look, diagram classes); no chapter text changed.
  docs.yml needed no change (the vendored copy is in the repo).

## Proposal

## Result
The guide's site looks like Elanor (D2200): Earendil brand tokens, Mallos Serif and Commit
Mono, Elanor's layout, prose spacing, code, tables, callouts, navigation and search,
light/dark with a toggle, the Earendil mark, and diagrams in token colors that follow the
page (and fall back to the same colors on GitHub). Elanor comes from its published
packages, vendored by `scripts/elanor-update.sh` (needs `npm login
--registry=https://pkg.earendil.com` once; `main` moves the pin, a second argument
switches the brand; then run `djs-docs regen`). The gate, `build` and Pages need no login.
`scripts/docs-overflow.sh` checks that nothing sticks out of its column.
Known gaps: the reference tables scroll sideways inside their box between ~1000 and
~1300 px (wide opcode names); the decision log is one long page with the same layout.
