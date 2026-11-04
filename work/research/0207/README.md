# 0207: where the bytes of plain dumps go

Plain dumps (format 3) measured on main at 33d94983 (with 0203's work so far). Tools:
[scripts/dump-size/dump-size.ts](../../../scripts/dump-size/dump-size.ts) (breakdown),
[estimate.ts](estimate.ts) (savings per idea). Full tables: [report.md](report.md),
[estimates.md](estimates.md).

## What was measured

| suite | dumps | raw | gzip -9 (each dump alone) | median raw | how |
|---|---:|---:|---:|---:|---|
| corpus | 1,197 | 24.0 MB | 5.07 MB | 14.8 KB | `djs-difftest --engine djs --plain-out DIR` (chains of ≤ 50) |
| corpus without the environment (below) | 1,197 | 8.6 MB | 2.54 MB | | |
| v2 + golden programs | 146 | 443 KB | 159 KB | 2.6 KB | `DJS_PLAIN_OUT=DIR cargo test -p djs-core --lib plain_dumps_at_every_idle_point` |
| pi suite (next mode, dump at every idle point) | 147 | 124 KB | 65 KB | 838 B | `pi-tests.ts --next --dump-every-idle --plain` with a 2-line write in `afterIdle` of js/pi/vm.ts (not committed) |
| sleep-dump | 1 | 2.7 KB | 1.1 KB | | examples/sleep-dump/out/sleep.dump.js |

The tool splits each dump by its structure (top-level statements, scope factories and
their members, deferred groups `$dump.gaN`) into categories: `header`, `seed`, `helpers`
(`globalThis.$dump = {…}`), `builtin` (`$dump.bN`), `hostcall` (`$djs.resume`),
`reenact` (other `$dump.pN`), `cont` (continuations `KaN`, split into code, `$djs_*`
scaffold lines, indentation, blank lines), `origin` (`FaN`), `record` (the `$djs`
property, by field), `activation`, `operand` (`aN_k`), `fn` (function texts of closures
and global functions), `scope` (factory wrappers), `data`, `globals`, `fixup` (`name`
fixups, freezes, `keep`), `patch`, `final`. The marginal gzip of a category is
gzip(dump) − gzip(dump without it).

**The corpus is mostly test environment.** 64% of its bytes (50% of gzip) are
`env.*`: the difftest prelude's state (its functions, scopes, globals) and the
codemode-harvest harness (its function, whose text holds the synthetic tool results,
plus the result queues and fake tools). A pi session in the next mode dumps neither:
prelude-next is rerun by the host and results come from tools. The pi suite's dumps show
what a real session's dump consists of; the corpus without `env` adds bigger scripts.

## Where the bytes go

Shares of raw bytes (marginal gzip in parentheses):

| category | pi | sleep | v2 | corpus w/o env |
|---|---:|---:|---:|---:|
| continuations (`Ka`) | 21.5% (10%) | 27% (16%) | 21% (13%) | 20% |
| origins (`Fa`) | 19.0% (9%) | 16% (4%) | – | 10% |
| `$djs` records | 17.3% (19%) | 15% (16%) | 18% (18%) | 6% |
| host calls | 11.6% (9%) | 3% | 4% | 10% (harness `later(result)`) |
| final statement | 9.4% (6%) | 3% | 3% | 2% |
| header + seed + helpers | 10.3% (11%) | 21% (22%) | 13% | 3% |
| operands + activations | 8.3% | 4% | 5% | 6% |
| function texts (`fn`) | – | 6% | 20% | 4% |
| fixups (`name`, freeze) | 0.6% | – | 4% | 12% (prelude functions' `name`) |

Per dump, a pi dump is ~170 B of fixed cost (header, seed, helpers, final statement),
~150 B per suspended activation's record, and the script about twice: once as the
continuation, once as the origin.

## The waste

1. **The origin repeats the script.** `Fa` is written when the original function is not
   otherwise in the dump: the codemode body (and callbacks whose closure is gone, e.g. a
   `.map(async p => …)` arrow). It is the whole function text, but the re-dump needs
   only what can still run, and the continuation already holds the verbatim rest:
   in the corpus 41% of origin bytes are stretches the continuation copied (`segs`),
   29% is the dead prefix before the earliest offset the record refers to. In v2,
   where origins are live global functions, the origin costs nothing: that is the model
   to aim at. sleep: the origin (437 B) repeats `step` (also `$dump.n2`) and the body.
2. **Records are verbose.** `Object.defineProperty($dump.Ka7, "$djs", { value: { f: …, at:
   "await#1", segs: […], scaf: […], cmap: [] } });`: 70 B of wrapper plus default fields
   (`at: "await#1"` in nearly all, empty `cmap`/`scaf`/`segs` often). Records gzip worst
   (numbers), 19% of pi's gzip.
3. **Dead frame variables.** Continuations declare every binding of the frame, also ones
   the rest never reads: in pi typically the body's `tools` and `console`
   (`var tools = $dump.a7_0;` plus the operand `$dump.a7_0 = tools;`). 6.7% of pi bytes in
   the declarations alone, ~10% with their operands; data only they reach comes on top.
4. **Same function, same site, written per activation.** `Promise.all(paths.map(async
   path => …))` suspended in six activations writes six continuations and six records
   that differ only in `path` and the awaited promise (h064: `$Ka15`…`$Ka20`).
5. **Name fixups.** Closures written as factory members (`$f33: (args) => …`) get the
   member's name, so each anonymous one is followed by `Object.defineProperty(f, "name",
   { value: "" })` (55 B). `(0, (args) => …)` keeps them anonymous for 5 B.
6. **Names and helpers.** `$dump.` occurs everywhere (3.6% of corpus w/o env, 5.6% of pi
   if it were `$d.`); `$djs_` scaffolding names; the `L`/`advance` helpers are written
   for reading (sleep: 18% of the dump is helpers).
7. **Not waste:** whitespace and blank lines (< 0.3%; the copied code keeps the
   script's own layout), builtins (0.1%), seed and header (needed).

## Proposals, ranked

Savings from estimates.md (raw; gzip where simulated). "pi" is the pi suite, "corpus" the
corpus without `env`.

### Clear wins, no format or readability trade-off

| # | change | pi | sleep | corpus | where |
|---|---|---:|---:|---:|---|
| A | Leave out the record's default fields (`at: "await#1"`, empty `segs`/`scaf`/`cmap`); the reader takes missing as default | −2.7% (gz −3.0%) | −1.5% | −0.9% | record writer and reader (dump_plain / dump_origin) |
| B | Anonymous factory members as `(0, f)`, no `name` fixup | −0.5% | – | −8.2% (mostly prelude functions) | scope member writer |
| C | Drop frame variables the continuation's own text never reads (conservative identifier scan of K; keep all with `eval`/`arguments`/`with`); their operands go with them | ~−10% | −1% | −3% | continuation writer (dump_cont) |
| D | `globalThis.$dump = {  };` → `{}` and similar trivia | <0.5% | | | |

C is the one with real substance (it also reads better), but it is in dump_cont.rs, where
pjr works; I would sequence it after a check with pjr.

### Trade-offs for the owner

| # | change | pi | sleep | corpus | cost |
|---|---|---:|---:|---:|---|
| E | Origin without its dead prefix: the statements before the site that no loop re-enters become declaration stubs (`const t0 = void 0;`, `async function step() {}`) | −4.5% (gz −2.8%) | −7% | −3.0% | the origin no longer shows the whole script; exporter must compute sites/segs on the pruned text (dump_origin/dump_cont) |
| F | Origin as text pieces in the record, the verbatim stretches as references to the continuation's own source; the exporter compiles the origin text when it dumps again | −5% (with E: ~−10%) | −7% (with E: ~−14%) | −4.2% (with E: ~−7%) | origin unreadable (a string); a compile at re-dump; large change in dump_origin |
| G | Host-provided origin for the codemode body (`f` names the body; the host that re-dumps supplies its source) | −19% | −16% | −10% | the dump is no longer self-contained for re-dumps (running it, in djs or node, still is) |
| H | One continuation per (function, site): continuations take their frame values as arguments, one record per K | small | – | −1.2% (h064: 5 of 6 copies) | K reads less directly; format change in dump_cont |
| I | `$dump` → `$d`, `$djs_` → `$j` | −5.9% (gz −1.1%) | −5% | −3.9% (gz −0.7%) | shorter, less self-describing names |
| J | Helpers written compactly | – | −5% | −0.2% | helpers less readable |
| K | Minified mode | | | < 1% | not worth it: the copied code keeps the script's layout, gzip gains nothing |
| L | Records as a helper call `$dump.o(K, f, segs, …)` | −9% (worse) | | −0.5% raw, +1.6% gzip | rejected: the helper costs more than it saves |

### Test environment (no format change)

- The size score should not be dominated by the difftest prelude and the harvest
  harness. Either the score counts the corpus without `env` bytes (the tool can), or
  the difftest harness keeps its prelude out of the dump as pi's next mode does (host
  globals), which also makes the plain-dump gate leg ~3x faster to read. I propose the
  former for the score (tests/scores/dump-size.tsv: raw and gzip totals of corpus w/o
  env, v2, sleep).

Realistic total for pi-like dumps: A+B+C+D ≈ −13%; with E and I ≈ −25%; with G or F
≈ −35%. The script appears twice by design (continuation + origin); the origin is the
one big lever, and every way to shrink it trades readability or self-containment.
