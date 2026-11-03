# 0185: Update the guide's design to the latest Elanor (fonts in particular)

status: open
agent: -
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

## Result
