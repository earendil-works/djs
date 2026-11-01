# 0115: The guide must be self-contained

status: open
agent: -
branch: issue/0115-guide-self-contained
depends-on: 0113
guide: all pages
touches: docs/guide/ (prose), crates/djs-docs/ (the check rule, and the decision page if added), AGENTS.md ("Keeping the guide current"), docs/guide/README.md (the authoring guide)

## Goal
Owner's rule: the guide is documentation for humans, so it must not lean on files that
aren't part of it. Today it references ARCH.md, TESTING.md and the decision log, and cites
the goals by number ("G2"), which a reader of the site can't resolve. Coordinator count on
main (2026-11-01): ~24 mentions of ARCH.md/TESTING.md/decisions.md/G1–G5 across the pages,
and ~135 links that leave docs/guide via ../../.

## The rule (to write into docs/guide/README.md and AGENTS.md)
- Anything the guide relies on is explained in the guide. In particular the goals: add a
  short "Goals" section/page (what djs optimizes for, and why), and refer to goals by name
  and link to it, never as "G2".
- No references to repository files outside docs/guide as a source of explanation
  (ARCH.md, TESTING.md, docs/*.md, work/…). Whatever a page needs from them is written in
  the page (or another guide page).
- Decisions: the guide must explain the decision itself in words. Decision IDs may appear
  only as clickable links to a page **inside the site** (for example a generated
  "Decision log" page built from docs/decisions.md, or per-decision anchors), never as bare
  references to a file the reader doesn't have. Decide which (propose to the coordinator).
- Source code: "Where to look" lists may point readers at code, but as links to the
  published source (the GitHub repository, at the commit the site was built from), clearly
  labeled as source, not as relative repo paths, and never as the place an explanation
  lives.

## Acceptance criteria
- [ ] All pages fixed per the rule; the goals explained in the guide.
- [ ] `djs-docs check` enforces it: it fails on links/mentions of repo markdown files
      outside docs/guide, bare goal numbers, and decision IDs that don't resolve to a page in
      the site; source links go through one helper so their target is consistent.
- [ ] docs/guide/README.md and AGENTS.md "Keeping the guide current" state the rule.
- [ ] Coordinate with the design agent (0114), who is restyling the site in djs-docs at the
      same time: this issue changes prose and the checker, not the look.

## Log

## Follow-ups

## Result
