# 0228: Publish the host package to npm as @earendil-works/djs 0.1.0

status: open
agent: -
branch: issue/0228-npm-package
depends-on: -
guide: js-api.md (installing and loading), introduction, cli? (no), the wasm boundary (module loading)
touches: js/ (package.json, build step, src loading paths), scripts/ (a package build script), .github/workflows/ (publish workflow), LICENSE files, README for npm, tests for the packed tarball, docs/guide/js-api.md

## Goal (owner decisions D2842)
Make `js/` publishable as `@earendil-works/djs` version 0.1.0 and set up the release pipeline,
ending at a dry run; the owner does or approves the first real publish.

1. **JS build:** emit JavaScript + `.d.ts` from js/src (tsc emit or strip-types + declarations);
   `exports`/`types` point at the built files. Node refuses to strip types under node_modules, and
   bundlers/browsers need JS.
2. **wasm in the package:** the shipped module (default entry `.`), slim (`./slim`) and dump
   (`./dump`) modules, built by scripts/build-wasm.sh (release, build-std as today), loaded relative
   to the package (`new URL("./djs.wasm", import.meta.url)`), working in node, Bun, Deno, browsers
   and bundlers (Vite/webpack asset patterns); DJS_WASM / `opts.module` still override.
3. **Package hygiene:** remove `private`, name/version, `files` whitelist, `repository`,
   `homepage` (the docs site), `license` with LICENSE-MIT and LICENSE-APACHE files (add them to
   the repo root too: the license field says MIT OR Apache-2.0 but no files exist), an npm README
   (what it is, install, a minimal example, link to the guide), `engines` verified. Drop the `./pi`
   export (pi's adapter stays in the repo for its tests). Keep `./wire`, `./reader`, `./dump-runtime`
   only if they are public API (decide and document).
4. **Release pipeline:** `.github/workflows/publish.yml` (on a version tag or manual dispatch):
   build wasm reproducibly (rust-src, the shipped build flags), build JS, `npm pack`, install the
   tarball in a clean directory and run the js tests and examples against it on node 20/22/24, Bun,
   Deno and a headless browser, then `npm publish --provenance --access public` (behind the tag /
   environment; needs the owner's npm token or trusted publishing). A gate leg (or a cheap check)
   that `npm pack` builds and the packed tarball imports and runs one script in node, so main can't
   drift from publishable.
5. Guide: js-api.md "Installing" section (npm name, entry points, module sizes, loading in browsers
   and bundlers).
Stop at `npm publish --dry-run` (with provenance where possible) and report the tarball contents
and sizes.

## Log

## Follow-ups

## Result
