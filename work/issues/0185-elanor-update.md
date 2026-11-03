# 0185: Update the guide's design to the latest Elanor (fonts in particular)

status: done
agent: design
branch: issue/0185-elanor-update
depends-on: 0114
guide: all pages (visual only)
touches: crates/djs-docs/ (assets/elanor via scripts/elanor-update.sh, the stylesheet, templates, highlighter roles if tokens changed, diagrams' colors via regen), scripts/elanor-update.sh (if the packages' layout changed), scripts/docs-overflow.sh (only to run it)

## Owner
"Update the docs design based on the latest Elanor (particularly font) stuff."

## Goal
The guide vendors Elanor pinned at ccdb0d9e (D2200; assets/elanor/VERSION). Move to the
newest Elanor main build with `scripts/elanor-update.sh main` (needs the Earendil npm login
once; if it isn't available, stop and tell the coordinator), then adapt the site to what
changed, especially the fonts: new families/weights/files, their licenses, @font-face
rules, font tokens, sizes/line heights, code font, and any renamed or new tokens (the
stylesheet's token test catches renames). Check light and dark, the live editor, diagrams,
code highlighting, tables and narrow widths in headless Chrome (before/after screenshots in
work/scratch/0185), and keep scripts/docs-overflow.sh clean. Record the update as a
decision, and note what Elanor changed in the issue log.

## Log
- 2026-11-03 (design): `scripts/elanor-update.sh main` with the existing Earendil npm
  login: ccdb0d9e -> d3184572 (3 upstream commits: #81 font sizes and weights for Mallos
  Serif 0.1.1, #82 form labels in the UI label style, #85 npm release/permanent snapshot CI).
- What Elanor changed: tokens.css only. Font files, icons, logos and colors.tsv came out
  byte-identical (Mallos Serif 0.1.1 was already vendored; the tokens are now tuned to it).
  All text styles a step smaller (prose 18->16, UI 16/14/12->14/12/10, label 16->14),
  heavier small UI text, lighter headings (app 500-600, marketing 450-550), selected item
  550, numeric primitive weights (bold -> 700, new 450/550), new
  `--semantic-number-font-scale-inline-code` (0.9375). No color, family, code-style or
  Shiki theme changes, so highlighter roles and diagrams are unchanged (regen: nothing).
  The token test passed as-is: the site used no renamed token.
- Site: inline code sized by the scale token (removed the 0.85em heading and 0.93em table
  overrides); tables, search results and the live buttons in ui-primary (14 px) instead of
  ui-secondary (now 12 px); footer in ui-secondary instead of the 10 px tertiary. Footer
  links both font licenses and the tokens' new ISC license (`assets/elanor/tokens.LICENSE`,
  copied by the update script). D2201.
- Checked in headless Chrome, before (old pin) vs after, in work/scratch/0185:
  values light/dark, opcodes table, gc dark, introduction with a live run, the live
  editor focused (overlay aligned, key hints), the footer, heap at 390 px.
  scripts/docs-overflow.sh: no overflow at 1440/1000/390. djs-docs check/test/clippy clean.
- Guide pages: none changed (visual only).

## Result
The site is on Elanor d3184572 (D2201): Elanor's retuned type scale for Mallos Serif 0.1.1
(16 px prose, 14 px UI, lighter headings, the inline-code scale token), with the tokens'
ISC license vendored. Colors, fonts files, highlighting and diagrams were unchanged
upstream. No overflow at any tested width.

## Follow-ups
- When Elanor tags a release (`vX.Y.Z`) or a maintainer keeps a main build
  (`main-<12 hex>`), elanor-update.sh could pin that permanent label instead of a SHA, so a
  fresh install keeps working after 30 days; today it accepts SHAs and `main` only.
