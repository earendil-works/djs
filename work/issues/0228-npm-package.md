# 0228: Publish the host package to npm as @earendil-works/djs 0.1.0

status: done
agent: npm
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

- Entry points (D3106): js/src/index.ts, slim.ts, dump.ts re-export sandbox.ts and override
  `loadModule`/`defaultWasmPath`/`Sandbox` (subclass with a static `defaultModule()`), each with a
  literal `new URL("../wasm/NAME.wasm", import.meta.url)`. sandbox.ts: compiled modules by
  location (the sync constructor's default), `create`/`resume` via `this`, `defaultWasmPath()` =
  `$DJS_WASM`, else the repo build only from source (non-literal so bundlers skip it); env.ts reads
  env safely in Deno; strings are fetched in browsers; fetch errors are SandboxErrors. Fixed the one
  tsc error (hookError narrowing); js/tsconfig.json checks clean now (dump-runtime.d.ts).
- Build (D3105): scripts/npm-pack.sh (tsc 5.9.3 pinned in js/package-lock.json, stage in
  target/npm/package, specifier check, npm pack). Packaging tests (D3108): js/pkg-test/smoke.js +
  scripts/npm-test.sh {node,bun,deno,js-tests,browser}. Verified locally on the release modules:
  node 20, 22, 26, Bun 1.4, Deno 2.9 (needs --allow-read), js/test on the package (98/98), Chromium
  native modules + Vite 8 build + webpack 5 build; TS consumers (nodenext and bundler) type-check.
- Exports (D3107): `.`, `./slim`, `./dump`, `./dump-runtime`, `./wasm/*.wasm`, `./package.json`;
  dropped `./pi`, `./wire`, `./reader`. LICENSE-MIT / LICENSE-APACHE at the root and in the package.
- .github/workflows/publish.yml (D3108); gate leg `npm package` (D3109), ~4 s.
- Docs: guide js-api.md (new "Installing", "Loading the module" rewritten, internal modules no
  longer called exports), introduction.md (npm line), dumps.md (name); README.md, docs/host.md
  (package section), TESTING.md (gate leg + "The npm package"), docs/snapshot-versioning.md
  (name). djs-docs strip.mjs skips `.d.ts`; paths.rs test names the new package.
- `npm publish --dry-run` (local, release modules): 29 files, 1.6 MB packed, 3.9 MB unpacked:
  wasm/djs.wasm 953 KB, djs-slim.wasm 825 KB, djs-dump.wasm 1.84 MB, dist/sandbox.js 86 KB
  (+37 KB .d.ts), the rest under 30 KB each.

## Follow-ups

- Owner: confirm the MIT copyright holder (LICENSE-MIT says "Earendil Works Co.", as pi-draw;
  pi-review says "Earendil Inc.") and the homepage URL (https://earendil-works.github.io/djs/,
  the default Pages URL of the repo).
- Owner, before the first publish: create the `npm` environment (required reviewers, tag rule),
  register trusted publishing for earendil-works/djs + publish.yml on npmjs.com (or add an
  NPM_TOKEN secret to the environment), then push tag v0.1.0. The workflow itself has not run on
  GitHub yet; its steps ran locally.
- The dump module is the biggest file (1.8 MB of 3.6 MB of wasm); a host that never dumps still
  downloads it with npm (bundlers copy only what is imported).

## Result

`js/` builds and packs as `@earendil-works/djs` 0.1.0 (scripts/npm-pack.sh → target/npm/
earendil-works-djs-0.1.0.tgz) with JS + .d.ts and the shipped, slim and dump modules loaded
relative to the package; scripts/npm-test.sh tests the tarball in node/Bun/Deno/Chromium (native,
Vite, webpack) and runs js/test on it; publish.yml publishes on a version tag after the owner's
approval; the gate's `npm package` leg keeps main publishable. Stopped at `npm publish --dry-run`.
