# 0249: Remove design attribution from the guide footer

status: done
agent: docs-footer
branch: issue/0249-remove-design-footer
depends-on: -
touches: crates/djs-docs/src/site.rs
guide: -

## Goal
Remove the Elanor, fonts and icons attribution paragraph from the documentation site's footer, as requested by the owner. Keep the vendored assets and their license files intact.

## Acceptance criteria
- [x] The landing page and chapter footers retain only the project and source links.
- [x] Bundled license files remain unchanged.
- [x] Site tests and formatting checks pass.

## Log
- Removed the attribution paragraph and its unused version parsing code.
- Added a regression test for both the landing page and a chapter.
- `scripts/timeout.sh 120 cargo test -q -p djs-docs site::tests`: four tests passed. `cargo fmt --all -- --check` and `git diff --check` passed.
- No guide prose pages are affected; the change is confined to the shared site template.
- Prepared a single-commit branch for the requested squash merge through `scripts/merge-to-main.sh`.

## Follow-ups
None.

## Result
The shared footer contains only the Earendil and GitHub project links. The licenses continue to ship with their assets.
