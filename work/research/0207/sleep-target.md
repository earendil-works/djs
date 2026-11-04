# sleep-dump against the target

The sleep-dump example's dump, produced natively by the core test `the_sleep_example_dumps`
(the Sandbox's shape: codemode body with `ms`, `console.log` and async `tools.sleep`), is
in the size score as `sleep_raw` / `sleep_gzip` (tests/scores/dump-size.tsv). The target is
the coordinator's hand-written sketch, work/research/0207/target-sleep.js.

| | raw | gzip -9 |
|---|---:|---:|
| 0207 start (README dump, before A/B/(c)) | 2,751 | – |
| after (c), D2836 (main cdb30b72) | 2,321 | 938 |
| target-sleep.js | 736 | 415 |

Where the 2,321 bytes go (scripts/dump-size `--show`, by category), and which item removes
them:

| bytes | category | what | removed by |
|---:|---|---|---|
| 437 | origin | `$dump.Fa6 = async function (ms) {…}`: the body's whole text again | (d) `$djs.source(ref)`, 4 (records) |
| 427 | cont.code | the continuations' copied text and declarations | stays (the program) |
| 278 | cont.scaf | `$djs_L` loop scaffold, `$djs_t0`/`$djs_R` temps, helper captures | 1 (temps), 2 (`$djs.rest`), 3 |
| 392 | record.* | `Object.defineProperty(K, "$djs", {…segs, scaf, cmap})` ×2 | 4 (canonical K: `{f, at}` or nothing) |
| 151 | fn | `$dump.n2 = async function step…` | 3 (a top-level `const step = …`, names only where referred to) |
| 81 | final | `try { ($dump.a6); } finally { delete … }` | 3 (the IIFE's value is the completion) |
| 78 | iterator | `$djs.advance(Array.prototype.values.call([...]), 1)` | stays (target keeps it, shorter as `[...].values()`) |
| 76 | activation | `$dump.aN = $dump.KaN.call(undefined)` | 3 (IIFEs) |
| 73 | hostcall | `$djs.resume(2, "tools.sleep", [5000], {...})` | stays |
| 65+35 | builtin, operand | `$dump.b0 = …["next"]`, `$dump.a6_0 = Array.prototype.push` | 1 (D2836: builtins written as their expression) |
| 39+14+23+12 | seed, header, `$dump = {}` | fixed | `$dump = {}` goes when nothing uses it (3) |
