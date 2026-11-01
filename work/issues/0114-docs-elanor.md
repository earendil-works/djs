# 0114: Align the guide's design with Elanor (Earendil's design system), using its published packages

status: open
agent: -
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

## Proposal

## Result
