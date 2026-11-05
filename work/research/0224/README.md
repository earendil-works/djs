# 0224: the native performance regressions since 2026-11-01

0223 found six benchmarks worse than their 2026-11-01 records. Each was confirmed, bisected
over the 1,912 commits since 0394842cf4 (a release `djs-bench` per commit in a scratch
worktree: 16 evenly spaced samples, then `git bisect run` with a threshold halfway between
the good and bad values), and fixed or explained.

The machine was rarely quiet (load 3.5 to 40, from 0225's builds and other work), so
timings were checked against **instruction counts**: `/usr/bin/time -l` instructions
retired by a small harness (work/scratch/0224/icount) running a fixed amount of work
against a given commit's djs-core, minus the same harness with zero repetitions.
Instruction counts repeat to well under 1% under any load.

## Results

| benchmark | 0223 said | cause (commit) | verdict | fix |
| --- | --- | --- | --- | --- |
| regexp/compile | +26–83% | e354b3664d (0050, STAR loops' follow sets) | real | follow sets only where a STAR uses them; fewer allocations |
| wire/stringify 10k-objects | +48% | ddd792d6f2 (0101, mark-compact) | real | `Arena::truncate` moves the zeroed frontier back |
| gc per MiB live | +60–66% | 19c7ebca8f, 08038ae8d2 (denser objects) + ddd792d6f2 (mark-compact) | mostly the metric | per-collection metric; slot plans; fused update and slide |
| regexp/scan UTF-16 | −14–27% (one run) | none | noise | none |
| wasm/instantiate | 29 → 68 µs | the bench's module (data segment) | not a code regression | also measure the shipped module |
| engine/djs/empty | 0.005 → 0.131 ms | 681e323c36 (0044, the prelude runs in every job) | measures something else | documented |

### regexp/compile

Instructions per compile (harness):

| pattern | 0394842cf4 | 1b3a318f45 (before) | e354b3664d | HEAD 23b0a946 | branch |
| --- | ---: | ---: | ---: | ---: | ---: |
| `/\d+/` | 7,558 | 7,581 | 7,868 | 8,534 | 5,848 |
| `/[\w.+-]+@[\w-]+\.[\w.]+/` | 20,220 | 20,379 | 27,787 | 31,344 | 21,190 |
| `/https?://([^/\s]+)(/[^\s?#]*)?/` | 31,320 | 31,301 | 44,351 | 49,566 | 31,691 |
| `/(?<y>\d{4})-(?<m>\d{2})-(?<d>\d{2})/` | 23,192 | 23,107 | 30,090 | 33,118 | 24,525 |
| `/\bfox\b/i` | 10,601 | 10,592 | 18,290 | 18,911 | 8,809 |
| `/^\s+\|\s+$/g` | 22,639 | 23,038 | 26,403 | 29,587 | 17,304 |

Allocations per compile, HEAD → branch: 12→8, 40→25, 64→36, 53→38, 27→10, 35→20 (1b3a318f45:
11, 24, 36, 37, 13, 27).

Cause: `Compiler::concat_follows` computed the start set (`start_cond`/`first_chars`: a
`CharSet` allocation each, case closures under `i`) of every element of every forward
concatenation, and cloned the accumulated set per element, whether or not any `STAR`
loop would read it. `/\bfox\b/i` has no loop at all and doubled. Malloc was 58% of the
compile profile.

Fix (crates/djs-core/src/regexp): `wants_follow` (a `STAR` reachable through groups,
alternatives and concatenations, exactly where `Compiler::node` passes the follow set
on); concatenations without one skip the computation, and the others start at their
first such element; the accumulated set moves into its slot instead of being cloned;
the STAR's disjointness test (`F_DISJOINT`) uses `CharSet::is_disjoint` and the body's
own set; the follow ranges are not copied into a Vec; `CharSet::add_range` inserts or
widens in place and splices only when it merges several ranges; `\d`, `\s`, `\w` are
built from sorted ranges; the program's words are reserved (128) up front. The
bytecode is unchanged: 5,033 patterns from test262 and the corpus compile to identical
words (work/scratch/0224/dump-*.txt).

What remains above the 2026-11-01 numbers (email +4%, date +6%) is the start-set
analysis the STAR loops need, which pays for itself in scans (`\d+` scan −28%
instructions since then).

Guard: crates/djs-core/tests/allocs.rs counts allocations per compile of the bench
patterns (a counting global allocator, per thread) against ceilings; HEAD's compiler
fails it on every pattern.

### wire/stringify 10k-objects

Cause: the in-place collector (ddd792d6f2) ends with `Arena::truncate`, which zero-filled
the freed tail and left it as the arena's *zeroed frontier* (the Vec's length). After
that, every allocation landed below the frontier, and `alloc_from_iter` (the UTF-16 and
narrowing string allocations) writes there byte by byte through a `zip` over a
`flat_map`, instead of the `extend` it uses at the frontier. stringify's spills of its
wide buffer showed 33% of the time in `alloc_utf16`, absent before. Cheney had left a
fresh to-space whose frontier was the live end.

Fix: `truncate` truncates the Vec (capacity kept). The arena re-zeroes 16 KiB ahead as it
allocates, so nothing reads stale bytes, and every collection saves a memset of all its
garbage.

| (native, ms) | 2026-11-01 | HEAD | branch |
| --- | ---: | ---: | ---: |
| stringify 10k-objects | 2.29 | 3.38 | 2.54 |
| stringify 1mib-text-unicode | 0.93–1.95 | 1.21 | 0.75 |
| stringify tool-results | 0.69–0.94 | 0.93 | 0.66 |

(Branch values from a run at load ~6. The final records are in tests/scores/bench.tsv.)

Guard: `arena::tests::truncate_moves_the_frontier_back`.

### gc

The benchmark divides the time of a full collection by the live MiB. The same 200k
records took 60.0 MiB at 0394842cf4, 47.8 MiB from 19c7ebca8f and 34.1 MiB from
08038ae8d2 (objects got denser), so ms/MiB rose while each collection did not get
slower. Instructions per full collection of the 200k records (harness):

| commit | records | linked list (500k) |
| --- | ---: | ---: |
| 0394842cf4 (2026-11-01) | 379M | n/a |
| b4beb2cec7 | 294M | n/a |
| 50fdc8e369 (Cheney, before 0101) | 296M | 571M |
| ddd792d6f2 (mark-compact) | 394M | n/a |
| HEAD 23b0a946 | 350M | 724M |
| branch: slot plans | 270M | n/a |
| + inline mark-stack push | 262M | 611M |
| + update pass reads one header, skips leaves | 248M | 611M |
| + update and slide in one walk | 238M | 593M |

Profile at HEAD: the generic `walk::for_each_slot` (matching every field of the layout,
per cell) was 59% of a collection in the mark and the update passes. The compactor's
old-region walk already had per-kind `SlotPlan`s; they moved to walk.rs and drive
`for_each_slot` everywhere (mark, update, Cheney, compaction). The fused pass: the
forwarding address needs only the bitmap and the block table, so a run of adjacent live
cells is updated cell by cell and then moved; each cell is touched once. Cycles per
collection, prototype vs the previous step (interleaved, loaded machine): records
45M → 34M, list 87M → 75M. Templates' inline-cache offsets (D1621) are now emptied when
the marker traces the template: the walk may already have moved the cache cell (test
`vm::tests::a_template_above_its_moving_cache_still_has_it_emptied`, which fails if the
emptying stays in the walk). The heap bytes after a collection are identical to HEAD's
(hash of a mixed heap with ropes and garbage), the difftest with suspension, GC stress
and determinism, e2e's gc and suspend modes and native-vs-wasm all pass.

Guard: the bench now reports `gc/<workload> collection` (ms for the fixed workload)
next to the per-MiB value, from the same samples.

### regexp/scan on UTF-16

Instructions per scan, 0394842cf4 → HEAD: zebra 4.56M → 4.55M (utf16) / 4.42M → 4.66M
(latin1), lazy dog 8.73M → 8.25M, fox 8.94M → 8.17M, `\d+` 17.4M → 12.5M. The timings
are bimodal on both builds (0394842cf4's `/zebra/g` latin1: 631, 646 and 943 MB/s in three
runs), from core placement or alignment. No regression.

### wasm/instantiate

The wasm group instantiates its own `compiled-prelude` build, whose data segment (515 KB)
carries what building a realm from scratch needs. Instantiation in node is memory setup
plus copying the data: synthetic modules with 10 pages took 20 µs empty, 35 µs with 116 KB
of data and 58 µs with 515 KB. The shipped module (116 KB of data) instantiates in about
40 µs in the bench (`wasm/instantiate shipped`, new). Smaller data is 0225's area.

### engine/djs/empty

Since 681e323c36 (0044), djs-difftest's djs engine runs the shared prelude in every job,
so `empty` is a VM plus the prelude: 0.148 ms there, 0.131 ms now. The 0.005 ms records
were a VM without the prelude. Documented in crates/djs-bench/src/engine.rs.

## The shipped module (wasm, what pi runs)

wmeasure (instructions of the whole node process minus an empty run, n = 3), base
23b0a946 → branch:

| workload | Minstr | Mcycles |
| --- | --- | --- |
| regexp-compile (new) | 5,353 → 4,649 (−13%) | 1,249 → 1,049 |
| json-objects (new, stringify) | 3,858 → 2,910 (−25%) | 747 → 598 |
| json | 1,224 → 1,177 | 305 → 291 |
| pi-like | 5,822 → 5,699 | 1,274 → 1,146 |
| arrays | 9,539 → 9,321 | 1,141 → 1,085 |
| strings | 2,237 → 2,190 | 388 → 381 |
| strbuild | 3,527 → 3,480 | 585 → 606 |
| higher-order | 7,846 → 7,459 | 1,118 → 1,073 |
| props | 7,133 → 7,126 | 1,009 → 1,024 |

Shipped module size: 1,033,538 → 1,034,541 bytes (+1,003: the slot-plan table and the
regexp fast paths).

## Tools

work/scratch/0224 (not committed): `run.sh` (bench at a commit), `bisect.sh` /
`bisect-step.sh` (git bisect run on a bench threshold), `icount/` + `icount.sh` (fixed-work
harness against a commit's djs-core, instructions and allocations), `prof.py` (self and
inclusive samples from a samply profile with `--unstable-presymbolicate`).
