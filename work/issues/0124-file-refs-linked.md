# 0124: Every file reference in the guide links to GitHub

status: open
agent: -
branch: issue/0124-file-refs-linked
depends-on: 0115
guide: all pages
touches: docs/guide/*.md (prose), crates/djs-docs/ (check, render: auto-linking), docs/guide/README.md

## Goal
Owner request: every reference to a repository file or directory in the guide (prose,
inline code like `crates/djs-core/src/heap/gc.rs`, tables, generated regions, diagrams'
captions) must be a link to that file on GitHub at the build commit (render::source_url,
D2102). Today only "Where to look" paths are enforced.

## Acceptance criteria
- [ ] `djs-docs check` finds every inline-code or bare-text token that names an existing
      repo path (file or directory, also `path:line` / `path#symbol` forms if used) and
      fails if it is not a link; code blocks exempt (decide for gen:excerpt headers: they
      should carry a link too).
- [ ] Either auto-link in the renderer (preferred if clean: inline code that is an
      existing repo path becomes a source link) or fix the sources; generated tables and
      regions link their paths too (fix the generators, not by hand).
- [ ] References to paths that don't exist (renamed/removed files) fail the check.
- [ ] Directories link to GitHub's tree view, files to blob view, at the build commit.
- [ ] Built site checked: count of path mentions vs links before/after reported;
      `djs-docs check` and the overflow script green.

## Log

## Follow-ups

## Result
