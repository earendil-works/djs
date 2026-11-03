# Snapshots across djs versions

Status: research report (issue 0140), for the owner. Nothing here is decided or built
yet; the demonstration and the measurements are reproducible with the scripts named
below. It updates the snapshot study of 0088 (work/issues/0088-snapshot-design-research.md)
with what was built since: compact containers (0117, D2260), the heap analyzer (0133,
D2264), and the base layout work (0136).

The owner's question: *how can snapshots be made stable if the builtins are no longer in
the snapshots? What happens if a new version of djs is released?*

## Since D026 (issue 0164)

The owner decided (D026) that a snapshot is keyed on what it depends on, not on the
engine build hash; 0164 built it (D2265–D2268). An image's roots cell carries a
dependency descriptor: the layout hash, the image feature set, a hand-bumped
`COMPAT_VERSION`, the opcode and intrinsic table hashes and `RECORDS_HASH` (the id tables
and record slot constants, read from the sources). A container (format 3) carries the
descriptor and the base image's builtin prefix hash instead of the build hash, and a
refusal names the component that differs. The build hash stays for what needs the exact
build: Sandbox journals and pi's fingerprint.

How often each changed, per issue on main up to 7074117e (`churn.py --deps --until=7074117e`
with the base image built at every merge since djs-prelude exists, `base-churn.sh`):

| | all 158 issues | the 129 with every base image built | the last 30 |
| --- | ---: | ---: | ---: |
| engine build hash (what images were keyed on) | 110 | 89 | 16 |
| cell layout | 24 | 18 | 2 |
| image features | 3 | 3 | 1 |
| opcode table | 13 | 10 | 1 |
| intrinsic table | 35 | 32 | 3 |
| internal id tables | 67 | 55 | 10 |
| base image prefix (content without the roots cell) | – | 54 | 9 |
| **an old snapshot survives:** under the build hash | 48 (30%) | 40 (31%) | 14 (47%) |
| a raw image under the descriptor | 80 (51%) | 66 (51%) | 19 (63%) |
| a container (descriptor and base prefix) | – | 61 (47%) | 18 (60%) |

So a snapshot survives about one and a half times as many merges as before: half of
them for raw images, a little less for containers, whose base image must also be the
same. The internal id tables are the component that changes most; 42 of its 55 changes (in
the middle column) came with a changed base image or table anyway, and some of the other
13 are constants that do not matter to images (a GC tuning constant, a visibility
change): the source reading is conservative on purpose. The intrinsic table is the next, since an insertion renumbers
every later id (the 307 moved ids below).

Not covered by the descriptor and still checked by hash: a pi session snapshot compressed
against pi's prelude image needs that image to come out byte for byte the same on the new
build, which any change to the compiler, the builtins pi's prelude touches or the
collector can break. That was not measured. The rest of this report predates D026; read
"the build hash" in it as what images were keyed on until then.

## The short answer

- **A djs snapshot has never been stable across builds, with or without the builtins in
  it.** An image is the engine's own memory: bytecode compiled by that build, builtin
  functions referenced by arena offsets and by numeric intrinsic ids, shapes, caches and
  cell layouts of that build. Since 0117, a snapshot no longer stores the builtins (the
  base image) or pi's prelude: it names them by hash and is compressed against them. That
  made snapshots 100x smaller without making them less portable, because the old builtin
  cells only mean something to the old engine. Between two builds five issues apart, 307
  of 450 intrinsic ids moved by one: an image from the older build, run on the newer one,
  would call `ResolvePromise` where it meant `PromiseResolveFunction`.
- **On an upgrade today, every snapshot of the old build is refused**, with a message that
  names both builds. Nothing is misread, and nothing is resumed (demonstrated below for
  the Sandbox and for pi).
- **Two cheap mechanisms make sessions survive an upgrade:**
  - *an engine registry*: keep the old module and its host code, finish the old sessions
    on them (drain), and start new sessions on the new build;
  - *journal replay*: rerun the script on the new build with the inputs it got
    (tool results, clock, seed). pi's durable code mode already does exactly this to
    survive restarts; across builds it worked unchanged in the demonstration. Only pi's
    fingerprint check and the Sandbox journal's engine and hash checks refuse it
    today, by design.
- **Making the images themselves portable** (migrating them, or a logical snapshot
  format) is possible but costs weeks plus a permanent compatibility contract on the
  compiler and the builtins. pi does not need it.
- **Recommendation:** replay is the durable record across versions, a snapshot is a fast
  path for the same build, and an engine registry with drain is added only if a
  deployment must continue sessions exactly across an upgrade. Details and phases at the
  end.

## 1. What a snapshot depends on

A snapshot (the container of D2260) is a 40-byte header, the host's metadata as JSON, and
the collected image, compressed with zstd against a *reference image*. Restoring needs:

| dependency | where | what changes it | issues that changed it: all 139 / the last 30 |
| --- | --- | --- | --- |
| the engine build hash | header, `Roots::BUILD`; checked first | any change to djs-core's sources, the generated builtins, cargo features, libm (D1330, D1750) | 100 (72%) / 14 (47%) |
| the reference image | named by kind and FNV-1a hash, not stored | the base image: the build; pi's prelude image: the build, the prelude and the start data (tools, globals, store, limits, D2262) | follows the build hash |
| cell layouts | every cell (`heap/cells.rs`, the layout hash) | a new or changed cell kind or field | 22 (16%) / 1 (3%) |
| opcodes and bytecode format | templates of every function, including the script's own and suspended frames' pcs | `bytecode/ops.rs` | 14 (10%) / 0 |
| intrinsic ids | builtin closures (promise resolvers, Promise.all elements), `call_intrinsic` in builtin code | `bytecode/intrinsics.rs`: any insertion renumbers every later id | 33 (24%) / 2 (7%) |
| realm slots | `Realm::INTRINSICS` by enum order | `interp/realm.rs` | 23 (17%) / 2 (7%) |
| the builtins themselves | ~3,700 cells of the base image (objects, templates, bytecode, tables), referenced by offset | `builtins/gen/prelude.js`, the compiler, object and shape code | prelude: 39 (28%) / 3 (10%); compiler: 33 (24%) / 2 (7%); objects and shapes: 29 (21%) / 2 (7%) |
| the base image's layout | which offset each base cell has | image ordering and compaction (`heap/compact.rs`, `vm/image.rs`, djs-prelude), the collector | 10 (7%) / 5 (17%); collector 12 / 3 |
| pi's adapter | host globals, deterministic version, prelude image recipe | `js/pi/vm.ts` | 10 (7%) / 3 (10%) |
| the container format | header layout | `js/src/snapshot.ts` | 2 / 2 |

(`python3 scripts/snapshot-research/churn.py` counts the issues on main that touched each
input; an issue is one merge to main, 139 so far.)

The build hash is meant to change whenever behaviour can change, so it changes with
nearly every engine change. Even the deeper formats change often while djs is developed:
the opcode table and the cell layouts have been quiet for the last 30 merges, but the
builtins, the intrinsic table and the base image's layout have not.

How much changes in a "typical release", measured between two real builds five issues
apart (N = main at 104f8a1e, the end of 0133; N+1 = main at a195956f, after 0136–0139),
with `djs heap` comparing N's base image structurally against N+1's:

| | |
| --- | --- |
| cell layouts, opcodes | unchanged (same layout hash) |
| base image | 314.4 → 314.8 KB; only 11% of N's 8-byte words sit at the same offset in N+1; 425 of 931 bytecode cells and 57 of 874 builtin functions were replaced; 365 templates and 341 function objects changed |
| intrinsic table | 450 → 451 entries: one insertion (`EachDense`) moved 307 ids by one |
| pi's prelude image | 480.1 → 469.2 KB; 11% of the words in place |

So even a release that keeps every format leaves an old image meaningless to the new
engine: its references into the base point at different cells, and its builtin
closures name different builtins. In the session below, the analyzer of build N+1 reads
the N session's builtin closures as `ResolvePromise`, `PromiseResolveFunction` and
`ThrowTypeError`; they are `PromiseResolveFunction`, `PromiseRejectFunction` and
`FunctionPrototypeBehavior`. That is what the build check prevents.

### "The builtins are no longer in the snapshot"

They never could travel on their own. A raw image that contained the whole base (as
before 0117) was just as pinned to its build: the cells of build N's builtins only run on
build N's interpreter, with N's intrinsic table and N's native functions. Leaving them
out changes where they come from (the module of build N, which also embeds the base
image), not whether they are needed. The same holds for 0135's proposal to move builtin
code out of the arena into a read-only segment of the module: snapshots would shrink
further, and they would still name the build whose segment they reference.

What *would* make builtins portable is identifying them by stable names (an intrinsic by
its name, a builtin function by its path from `globalThis`) instead of by offsets and
table indexes. That is one ingredient of image migration (option c) and is not enough on
its own.

## 2. Demonstration: an upgrade, end to end

`scripts/snapshot-research/upgrade.ts` runs two deployed versions side by side: build N
with N's own host code (`js/`) and module, build N+1 with this tree's. N = 104f8a1e (main
after 0133), N+1 = a195956f (main after 0139).

```sh
git archive 104f8a1e | tar -x -C OLD && (cd OLD && scripts/build-wasm.sh)
scripts/build-wasm.sh && node scripts/snapshot-research/upgrade.ts OLD
```

Output (abridged):

```text
build N   924678c5, base image 1e102777b4406413 (321960 bytes)
build N+1 b175525f, base image 9630adfcc7b7d309 (322368 bytes)

== Sandbox
snapshot on N: 17696 bytes
restore on N+1: SandboxError misuse: snapshot from djs build 924678c5; this module is
  build b175525f: an image restores only into the engine build that wrote it
restore on N (engine registry): note 99, count 1
recorded on N: ["team-5","team-6",true,"2026-01-01T00:00:01.000Z"]; journal of 6 entries, 755 bytes
Sandbox.replay on N+1: diverged: replay diverged: engine b175525f, the journal was made by 924678c5
semantic replay on N+1: same result as on N; calls ["alpha"] ["team-5"]

== pi
N, uninterrupted: done {"ok":true,"value":"{\"ok\":true,\"steps\":3}", ...}
  fingerprint N:   "djs 924678c5, prelude 8c6c9242, host 71699705, deterministic 2, ..."
N -> N+1 resumeVm: SnapshotError: snapshot from djs build 924678c5; this module is
  build b175525f: an image restores only into the engine build that wrote it
  the snapshot: 2091 bytes
N -> N (registry): same messages as the uninterrupted run
replay on N+1: same calls, arguments, output and result as on N (20.9 ms)
  fingerprint N+1: "djs b175525f, prelude 8c6c9242, host 71699705, deterministic 2, ..."
  journal: 2 results, 189 bytes
```

What it shows:

1. **The Sandbox** (state kept between runs: 100 notes and a counter): its snapshot from
   N is refused by N+1 with a clear message. With N's module and host code kept (an
   engine registry), it restores and the next run continues the state.
2. **Sandbox journals** (D1730) refuse another engine by design: a journal records the
   build and canonical heap hashes, which differ between builds even when the script
   behaves the same. A *semantic* replay (answer the async calls from the journal, check
   names and arguments, compare the result) reproduces the run exactly on N+1. The
   Sandbox has no such mode yet.
3. **pi** (snapshot-research's `ask` case, a deployment waiting for approval): suspended
   on N at its second tool call, a 2.1 KB snapshot. `resumeVm` on N+1 refuses it; on N
   (the registry) it finishes with exactly the messages of an uninterrupted run. A replay
   on N+1, the same start data and the same results in the same order, produces the same
   calls, arguments, output and result in 21 ms (a cold start, the prelude included).
   pi's durable code mode would still refuse that replay today, because its fingerprint
   contains the djs build hash (`djs 924678c5` vs `djs b175525f`).
4. **Why the refusal is right**: the N session's image, read by N+1's analyzer
   (`djs heap --diff prelude-n.img pi-n.img`), holds a promise resolve function and a
   reject function whose intrinsic ids belong to other builtins on N+1.

## 3. Options

Sizes and times are from today's builds (wasm profile, node 25): a waiting pi session's
snapshot is 2–9 KB, one holding tool data 58–404 KB (the data), an empty Sandbox 0.8 KB
since 0136; the module is 1.06 MB (458 KB gzip), its host code 148 KB (42 KB gzip).

### a. Engine registry with drain

Keep every build that wrote snapshots still alive: its djs-wasm module and its host code
(`@djs/host` and the pi adapter, since the host side changes with the ABI and the
adapter's recipe), keyed by the build hash every container already carries. Restore a
snapshot with its own build and let the session finish there (drain); start new sessions
on the newest build. Retire a build when no snapshot needs it (sessions done, or past a
retention window); keep a revocation list for builds with a security or correctness bug,
whose snapshots then fall back to (d) or fail.

- **Guarantees:** exact continuation, at any instruction boundary, of anything a snapshot
  can hold. Demonstrated above for the Sandbox and pi.
- **Cost:** the host loads modules by build hash and bundles host code per build (a
  dynamic import per version, or a version directory). No engine work: the build hash,
  the container header and the refusal exist (D2260).
- **Size:** ~500 KB gzip per retained build, once per deployment; snapshots unchanged.
- **Risk:** old bugs keep running until their sessions drain; a revoked build strands its
  sessions; every old host bundle must keep working in the current process (same
  JavaScript runtime). Drain only works when sessions end in bounded time.
- **pi:** the launchers would pick the module by the snapshot's build; pi's fingerprint
  check already pins a session to its engine.

### b. Self-contained or content-addressed references

Store the reference with the snapshot (or the diff to a published, content-addressed
reference) so that a restoring host need not rebuild it.

- **Guarantees:** the image can always be *decompressed*. It still only *runs* on its
  build, so this needs (a) anyway, and with (a) it is redundant: the base image is inside
  the module, and pi's prelude image is rebuilt from the module and the session's start
  data (byte for byte, D2262).
- **Size:** 70–85 KB compressed per reference (the base 70 KB gzip, pi's prelude image
  85 KB zstd -19): 10–40x a waiting session's snapshot.
- **Worth doing only** where the start data is not kept with the session (a store of
  prelude images keyed by hash would then replace it). Otherwise not.

### c. Image migration with stable builtin ids

A versioned image format and a migration for each release: map every cell of the old base
to its counterpart in the new one, renumber intrinsics by name, convert changed cell
layouts, and rewrite every reference.

- **What it needs from the engine:** builtins identified by stable names (intrinsics by
  name, builtin functions by path, realm slots by name) and a migration per layout change.
  The analyzer's structural matching (D2264) is a prototype of the base mapping: between N
  and N+1 it pairs most base cells, and it reports the rest as new or gone.
- **The hard part is code, not data.** A session's own bytecode (the script, pi's prelude
  closures, which are user code) was compiled by the old compiler, and suspended async
  functions hold pcs, operand stacks and local slots of that bytecode. Code from an older
  compiler must either keep running on the new interpreter (opcodes frozen) or be
  recompiled with every suspension point mapped (0088's stack maps). Builtin code
  replaced between versions (425 bytecode cells between N and N+1) has no counterpart;
  a session suspended inside a builtin cannot migrate at all.
- **Guarantees:** only as good as the migration code, and a wrong mapping misbehaves
  silently (the intrinsic example above) instead of refusing.
- **Cost:** weeks of engine work, then a migration for every format change, forever, with
  old images kept as test fixtures.
- **Not recommended.** It buys exact continuation across versions, which (a) gives for a
  fraction of the cost as long as sessions drain.

### d. Journal replay (0088's B)

Record what the session took from outside (the start data, every tool result in order
with its clock, the seed) and rerun the script on the new build.

- **pi already does this** to survive restarts: pi's durable code mode journals each
  result before handing it to the script and, after a restart, reruns the script from its
  start data, delivering the journaled results; nested tool calls reattach by key, so no
  side effect repeats, and a script that takes another path is refused. It only adds the
  rule that the VM fingerprint must not change, and the fingerprint contains the djs
  build hash.
- **Guarantees:** the same calls, arguments, output and result whenever the new build
  behaves the same for that script (demonstrated). When it does not (a bug fix, a changed
  error message that reaches a tool's arguments, other numbers from a libm update), the
  replay diverges and the divergence is detected before any new call is made.
- **Size:** the start data plus the results: 189 bytes for the demonstration, the tool
  results themselves for heavy sessions (0088 measured 0.3–2.4 KB for light sessions,
  58–296 KB zstd for heavy ones). pi stores the results anyway, in its nested calls.
- **Time:** the script's own computation again: 21 ms here including a cold start,
  1–43 ms for 0088's six cases. Bounded by the script's CPU time, not by how long it
  waited.
- **What djs needs:** a cross-version replay mode in the Sandbox (check calls, arguments
  and the result, not the engine and the canonical hashes, which only a same-build replay
  can check); golden journals of real codemode scripts replayed by every build in the
  gate, so that behaviour changes that would break replays are visible.
- **What pi needs:** a fingerprint policy that tolerates a new djs build (its
  divergence checks already catch what matters), keeping the module version and prelude
  checks strict.

### e. Logical snapshots (0088's C)

Serialize the JavaScript-level state (objects, closures by function and captured
variables, suspended async functions by source position and locals by name) independent
of the engine, and rebuild it on any build. 0088 measured 1–4 KB for waiting sessions and
estimated 6–10 weeks of work, with a permanent constraint: every change to how the
compiler lays out code around `await`, `for-of` or `try/finally` needs a migration, or old
snapshots stop restoring. Unchanged by this report; still not needed for pi.

### f. Hybrids

- **Snapshot as a fast path, replay as the record** (recommended): store the snapshot
  next to the journal. A restart on the same build restores the snapshot (no replay); on
  another build it replays the journal; a divergence is reported as today.
- **Registry for in-flight sessions, replay after retirement:** sessions continue exactly
  on their build during a retention window; afterwards, or for revoked builds, they
  replay.
- **"Image-compatible" release series** (keep layouts, opcodes, intrinsic ids, base layout
  and compiler output fixed within a series): not practical while djs develops. It would
  freeze nearly everything the churn table shows changing, and one forgotten change
  corrupts silently.

| | exact across builds | engine work | ongoing cost | snapshot size | failure mode |
| --- | --- | --- | --- | --- | --- |
| a. registry + drain | yes, until retired | none | keep ~500 KB/build, host bundles | unchanged | refused after retirement or revocation |
| b. self-contained reference | no (needs a.) | none | +70–85 KB per snapshot | larger | as a. |
| c. migration | yes, if correct | weeks | a migration per release | unchanged | silent misbehaviour |
| d. journal replay | same behaviour, not same image | small (replay mode, golden journals) | behaviour stability, checked in the gate | journal: inputs only | divergence, detected |
| e. logical snapshots | yes, within the format | 6–10 weeks | compiler and builtin compatibility | 1–4 KB | refused when a construct changed |
| f. snapshot + replay | same build exact, else d. | as d. | as d. | both | as d. |

## 4. Recommendation for pi

What pi needs, from its durable code mode (pi-private packages/durable): a code mode
script lives seconds to hours (it waits on tools, subagents or a human); its VM state
never outlives one tool call; after a restart it is replayed from journaled results; the
long-lived state (transcript, documents, `store`) is plain data. djs would be upgraded
with pi releases, so a session suspended across a deploy is the case to handle, and
pi's integration of djs is deferred (D021).

**Phase 0 (now, done):** the contract is explicit. A snapshot names its build and its
reference and restores only there; another build gets a clear refusal (D2260). This
report and its scripts document what changes between builds.

**Phase 1 (with pi's adoption of djs):** replay is the durable record across versions,
snapshots are the fast path.
- Store a session's snapshot next to pi's journal. A restart on the same build restores
  it (no replay, ~1 ms); on another build it replays the journal.
- pi: let a changed djs build hash through the fingerprint check (keep the module version,
  prelude and limits strict); its divergence checks stay in force.
- djs: a cross-version replay mode for Sandbox journals (calls, arguments and result
  checked; hashes only on the same build); golden journals from the harvested codemode
  corpus (0118) replayed by every build in the gate, to see behaviour changes that would
  break replays before they ship.

**Phase 2 (only if a deployment must continue sessions exactly across an upgrade without
replaying, e.g. hosts whose tools cannot be replayed):** an engine registry with drain:
keep module and host code by build hash for a retention window, a revocation list,
replay as the fallback after retirement.

**Not planned:** image migration and logical snapshots. Revisit if sessions become long
and compute-heavy enough that replay is too slow, and pinning to a build is unacceptable.

## Reproducing

- `python3 scripts/snapshot-research/churn.py [OLD NEW]`: the churn table; with two
  commits also the intrinsic ids that moved between them.
- `python3 scripts/snapshot-research/churn.py --deps [--until=REV] BASE.tsv...`: the
  table of "Since D026"; `scripts/snapshot-research/base-churn.sh $(python3
  scripts/snapshot-research/churn.py --base-revs) > BASE.tsv` builds the base image at
  each merge for it (about 25 s each, an hour for all of main).
- `node scripts/snapshot-research/upgrade.ts OLD [OUT]`: the demonstration (OLD: a
  `git archive` of an older commit with its djs-wasm built); OUT gets the base images,
  pi's prelude images, the snapshots and the decompressed session image.
- `djs heap --diff OUT/base-m.img OUT/base-n.img` and `djs heap --diff OUT/prelude-n.img
  --only diff,functions,session OUT/pi-n.img`: the base comparison and the session's
  builtin closures above.
