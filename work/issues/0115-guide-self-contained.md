# 0115: The guide must be self-contained

status: done
agent: docs
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
- [x] All pages fixed per the rule; the goals explained in the guide.
- [x] `djs-docs check` enforces it: it fails on links/mentions of repo markdown files
      outside docs/guide, bare goal numbers, and decision IDs that don't resolve to a page in
      the site; source links go through one helper so their target is consistent.
- [x] docs/guide/README.md and AGENTS.md "Keeping the guide current" state the rule.
- [x] Coordinate with the design agent (0114), who is restyling the site in djs-docs at the
      same time: this issue changes prose and the checker, not the look.

## Log

- Proposal (sent to the coordinator): keep the generated decision-log page
  (decisions.html, already in the nav, per-decision anchors `#dNNNN`) as the target of
  every D-ID; give it a reader's introduction; goals on a new page goals.md linked by
  name; one source-link helper at the build's commit; check enforces it all. D2102.
- Checker (render.rs): on SUMMARY pages, strict mode now also fails on links to `.md`
  files outside docs/guide (the log, `../decisions.md`, stays allowed), mentions of
  `*.md` files that are not guide pages, `work/…`, `ARCH`/`TESTING` and issue numbers
  (`0088`) in prose and inline code, bare `G1`–`G5`, and code paths under "Where to
  look" that are not links. Code blocks and the decision log are exempt. Tests:
  `the_guide_is_self_contained`, `decisions_and_links` (source link with class).
- `render::source_url`: every repository link is `blob|tree/<rev>/<path>` with
  `class="source"`; `<rev>` is `$GITHUB_SHA`, else HEAD, else main (site.rs
  `source_rev`).
- Generated text: opcode and intrinsic descriptions drop parenthesized references to
  documents and issue numbers (`without_dev_refs`); the status text no longer names
  TESTING.md, and an issue-number provenance shows only its date.
- Decision log page: its contributor preamble is replaced by an introduction for readers
  (site.rs `DECISIONS_INTRO`; the first entry is found as the first `## D<digit>`).
- New page goals.md ("What djs optimizes for", after the introduction): the five goals
  with why and where in the guide, the two constraints, the non-goals. introduction.md
  links the goals by name and points to it.
- Prose fixed on every page (mine and infra's): goals by name and link; ARCH/TESTING/
  docs/*.md/work references removed, with what the page needed written in (testing.md
  now lists the known differences; host.md the export count; determinism.md the
  wasm32 cap); "Where to look" paths are source links; issue numbers replaced by what
  they found. Paragraphs rewrapped to 88 columns (inline code kept whole).
- On request (perf's 0110, D2090): builtins.md's natives excerpt widened to 17 lines
  (the fused `n_dense_pre` check and the HasProperty resume point), the prose describes
  the fused check, D2090 added to the page's decisions.
- Rule text: docs/guide/README.md ("The guide is self-contained") and AGENTS.md "Keeping
  the guide current". D2102 in the log and the index.
- Follow-up after review (design archived): the header's "Source" link now goes through
  `render::source_url` at the build's commit (site.rs `template` takes the revision);
  no `blob/main` link is left in the built site (`$GITHUB_SHA` overrides, checked).

## Follow-ups

## Result

The guide is self-contained and `djs-docs check` enforces it (D2102): no links to or
mentions of documents outside the guide, no issue numbers, goals by name (new page
goals.md), decision IDs only as links to the site's decision log (which now opens with a
reader's introduction), code only as source links made by one helper
(`render::source_url`, at the build's commit, `class="source"`). The rule is in
docs/guide/README.md and AGENTS.md.
