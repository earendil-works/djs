# Property access: measurements and design (0062)

Status: both parts are built. Part 1 (B, inline caches) landed in 0062: D1620 (globals
and own properties), D1621 (prototype properties). Option A (cheaper misses) was not
needed for the hot sites and is left for the builtin language's `%Get` path, which 0063
mostly replaces. Part 2, shapes and dictionaries (C, merged with 0052 S2), landed in 0083:
16-byte property entries and AccessorPair cells (D1830), shapes with strong transitions and
a per-heap budget (D1831); §7 is its design and §7.11 its results. Sections 1-5 are the
original analysis, kept for the reasoning.

## 1. Measurements

Release build on an M-series Mac, a shared machine (other agents running), so numbers are
± 20%. Tools are in work/scratch/0062: `micro.py` (per-access costs), `classify.py` (samply
profiles bucketed by inlined stack frame), and `opcount.patch` (counts executed
instructions per opcode, user code vs the prelude; not committed).

### Per-access cost (ns per access, net of the loop; 10 accesses per iteration)

| access | djs | qjs |
| --- | ---: | ---: |
| own data property, 4 properties (linear table) | 9–14 | ~1 |
| own, 20 properties (hashed table) | 15–17 | ~0.5 |
| prototype, depth 1 / depth 3 | 5–23 / 24–26 | ~1 / ~1 |
| own write | 13–15 | ~0 |
| `arr.length` | 14 | ~0.5 |
| global read (`Infinity`) | 17 | ~0 |
| `Math.PI` (global + hashed own) | 29–99 | ~0.3 |
| method call `p.f()` (depth 1) vs plain `f()` | 22–90 vs 7–60 | 8 vs 6 |
| `str.charCodeAt(0)` (a prelude builtin) | 285 | 11 |
| `arr.push(i); arr.pop()` (two prelude builtins) | ~1,500 | 34 |

A property access costs 10–30 ns in djs and about 1 ns in qjs. That is 3–8 dispatched
instructions' worth, so it matters for property-heavy user code. The last two rows matter
much more: a call of a builtin method costs 0.1–1 µs.

### Where the time goes, by workload

Samples are bucketed by the innermost recognizable frame. "Property lookup" covers
`lookup_property`, `find_own`, `table_find`, `get_property`, `set_property`,
`get_element_fast`, globals and the rest of that machinery, including when an intrinsic
(`%Get`) does the lookup. "Prelude instructions" is the share of executed bytecodes that
belong to builtins written in the builtin language.

| workload | property lookup | GC | call/return | intrinsic calls | prelude instructions |
| --- | ---: | ---: | ---: | ---: | ---: |
| engine/props | 29% | 10% | – | – | small |
| engine/arrays | 13% | 1% | 9% | 6% | 62% |
| engine/float (`Math.sqrt`, `Math.min`) | 7% | 2% | 11% | 6% | 79% |
| engine/higher-order (map/filter/reduce) | 7% | 1% | 8% | 7% | 89% |
| pi-like codemode script (scratch `pi-like.js`, 4.8x qjs) | 3.5% | 5% | 9% | 6% | **97%** |
| test262 `decodeURI/S15.1.3.1_A2.5_T1` (Script-mode globals) | 17% | 6% | 15% | 8% | 41% |

The test262 slowest list is otherwise TypedArray `copyWithin` detached tests (26 s each,
worth looking at separately), decodeURI, and RegExp property-escape tests.

**Main finding:** in codemode-like scripts, almost all executed bytecode is the builtin
language. `arr.push` runs about 100 prelude instructions; a map/filter pipeline is
interpretation of `Array.prototype.map` and friends plus `%Get`/`%Set` intrinsic calls.
Faster property access cannot fix that. The lever is executing builtins natively, which is
0063 (the bgen Rust backend). Property-access work pays for user-level property code
(engine/props, class-heavy code), for globals (Script-mode test262, `Math`/`JSON`/`Object`
in codemode), and for memory and image size.

## 2. Constraints

- **Position-independent arena, copying GC:** object identities are offsets that change at
  every collection. Anything keyed by offset must be rehashed (D1381) or invalidated at GC.
- **Deterministic snapshots (D1090, D1601):** same history, same bytes. Caches may hold
  state, but it must be a function of the history, and it must never change semantics.
- **The JS reader (js/src/reader.ts)** reads objects straight from the arena: own keys and
  values. A new object layout means a new reader.
- **No user code in leaf paths (D1085):** caches only speed up what the heap does; misses
  fall back to today's code.
- **0063 (native builtins)** uses `get_element_fast`, `object_class`, `array_length` and
  `call_intrinsic`; those keep their contract under every option below.
- **Image and memory (0052 S2):** 24-byte PropEntry, 424 builtin functions with their own
  72-byte `length`/`name` table, an object with 4 properties is 144 B (a 24 B object plus a
  120 B table).

## 3. Options

### A. Cheaper lookups, no caches (micro-optimizations)

Today a lookup does, per object on the chain: a typed-array class check, `find_own` (index
key check, array `length` check, table probe), `read_loc` building a `Property`, then
`Lookup`/`Got` matching. Moving the exotic checks behind a class flag, comparing keys in a
tight loop over the entries, and returning the value directly in the common data case
should remove 30–40% of the lookup cost. Small and safe, but the ceiling is low: it is
still a search per access.

### B. Per-instruction inline caches without shapes ("verified index" ICs)

Each `get_field`/`get_method`/`put_field`/`get_global`/`put_global` gets a cache slot.
There are no shapes: the cache stores the property's **entry index** in the property table,
and a hit **re-verifies the key** at that index (`index < COUNT && KEY[index] == atom`, flags
data/writable). Objects with the same insertion order (one literal, one constructor) hit the
same index, so a site is effectively monomorphic per layout. Verification makes it correct
by construction: a stale cache is just a miss.

- **Own properties:** cache `index + 1`. A hit is a few loads and one compare; expected
  9–17 ns → ~3 ns.
- **Globals:** cache which table (the global lexical record or the global object) and the
  index. For a global object hit, a realm **lexical epoch** (bumped when a Script declares a
  global `let`/`const`) proves the lexical record still lacks the key. Expected 17–100 ns →
  ~3 ns.
- **Prototype properties and methods:** cache the receiver's prototype (an offset), the
  depth, the holder's index, and a **prototype epoch**. The epoch is bumped by every
  collection (offsets move) and by any key-set or prototype change on an object flagged
  `OBJ_IS_PROTO`; that flag is set once an object becomes some object's prototype. A hit
  checks the epoch, the receiver's own miss (free when it has no table, a ≤8-key scan
  otherwise), the prototype identity, and the key at the index. Expected 5–25 ns → ~4 ns.
- **Storage:** a per-template `U32Array` of 3-word slots (`FuncTemplate::IC`, allocated
  with the template). The compiler adds a `u16` cache index to the five ops: about +2 bytes
  per property op, a few KB of bytecode in the image, plus ~12 bytes per site at runtime.
- **GC and snapshots:** the cache is not traced. Indexes and epochs are offset-free, and
  offsets are only used together with the epoch, which every collection bumps. The epoch
  lives in the heap (Roots or Interp), so a restored VM continues identically.
- **Reader:** no change (templates are not read); `layout.generated.ts` changes.
- **Effort:** about a week. The heap stays as it is.
- **Expected gains:** engine/props −15–20%, float −20–25% (the `Math` global and method),
  Script-mode test262 with many globals −20–30%, pi-like codemode −2–4%.

### C. Shapes (hidden classes) as heap cells

The QuickJS/V8 model:
- An object points to a shared **shape**: its keys and flags, its prototype, and a
  transition table to child shapes. Its values live in a slot array (out of line, or inline
  with slack).
- Transitions can be weak: an EphemeronArray keyed by the child shape (D1382), so unused
  shapes die. They are deterministic per history.
- Dictionary mode (today's PropTable) takes over after deletes, many properties, or
  attribute games.
- ICs become `(shape, slot)`: one compare instead of a key check, real polymorphic caches,
  prototype checks folded into the shape. Prototype validity still needs a cell or epoch,
  as in B.

What it adds over B, in speed: about one load less per hit, and cheap own-miss proofs for
the receiver. In memory, it is the big win:
- **Per object:** 8 B per property value instead of 24 B per entry plus table slack. A
  4-property object goes from 144 B to ~56 B.
- **Builtins:** the 424 `length`/`name` tables collapse into one shape (≈ 25 KB of base
  image).

It absorbs most of 0052 S2: 16-byte dictionary entries, shared `length`/`name`. What it
costs:
- **The object layer** (heap/object.rs, 1,500 lines, and its many callers) is rewritten.
- **The reader** has to decode shape chains.
- **The wire format** and all heap tests change.
- **The snapshot format** changes.
- **Effort:** about 3 weeks, with a long tail of exotic objects (arrays' `length`, typed
  arrays, Proxy, String wrappers, functions' lazy properties).
- **Conflicts:** high risk of conflicts with 0063.

### D. Prototype method caches only

B's prototype part on its own. Without the own and global parts it misses the cheapest
wins, so it is not proposed separately.

## 4. Proposal

1. **Now (0062):** A + B.
   - Order: the global ICs (the biggest single win: test262 and `Math`/`JSON` in
     codemode), then own-property ICs for `get_field`/`put_field`/`get_method`, then
     prototype method ICs with the epoch.
   - Each part comes with before/after numbers (engine track, pi bench, test262 CPU time)
     and the usual checks (ratchets, the three e2e modes, difftest stress identical).
   - The object layout, the reader and 0063's heap API stay unchanged. Bytecode gains an
     IC index on five ops; FuncTemplate gains an IC field.
2. **Later, its own issue: shapes (C), merged with 0052 S2.** Sequence it after 0063 has
   landed. The case for it is mostly memory and image size (S2's 70–90 KB, per-object
   memory), with speed second. B's IC sites carry over: only the hit check changes, from
   "key at index" to "shape equals".
3. **Not property access, but the largest lever for codemode:**
   - Native builtins (0063). After it lands, re-measure with `opcount.patch` to see where
     the rest of the pi-like 4.8x goes.
   - The prelude `%Get`/`%Set` intrinsic calls in builtins that stay in the builtin
     language could get the same verified-index caches, keyed by call site.

## 5. Open questions

- The IC index as an immediate vs a pc-keyed direct-mapped cache (no bytecode change, more
  misses). I lean to the immediate.
- The epoch's home: Roots (with D1601's threshold) or Interp.
- Whether to cache misses (absence) for `get_field` of optional properties. Not in the
  first part.

## 6. Results of part 1 (0062)

LTO builds with one codegen unit, interleaved runs, CPU-time minimum of 7. The machine
was at load 8–30, so engine-track differences under ±10% are noise. Default 16-unit
release builds vary ±15% between builds just from code layout.

| access | before | after |
| --- | ---: | ---: |
| global read (`Infinity`) | 10.4 ns | 1.3 ns |
| `m.PI` (own, hashed) | 9.4 ns | 2.5 ns |
| own read, first of 4 | 6.5 ns | ~0–1.7 ns |
| own read, 4th of 4 | 14.7 ns | 2.4–5.8 ns |
| own write | 15.7 ns | 1.0–10 ns |
| prototype read, depth 1 / 3 | 13.7 / 16.3 ns | 2.8 / 2.6 ns |
| method call `p.f()` | 28.6 ns | 20.1 ns |

Workloads:
- engine/props: −14 to −18%.
- engine/arrays and engine/higher-order: −14 to −22% with the prototype caches
  (`arr.push`, method lookups).
- test262 decodeURI (Script-mode globals): −13%.
- The pi-like script and float: within noise, as predicted; their time is in the builtin
  language.

## 7. Part 2 design: shapes and the leaner layout (0083)

Status: proposal (0083), for review before implementing. Decision ids D1830+.

### 7.1 What we have and what we want

Today every object with properties owns a `PropTable`: a 24-byte header plus 24-byte
entries (key, flags, value, setter), with capacity doubling and a minimum of 4. So
`{a, b, c, d}` takes 24 B (object) + 120 B (table) = 144 B, and each of the 424 builtin
functions of the base image carries its own 72-byte `length`/`name` table (0052 S2). The
0062 caches verify a key at a table index on every hit.

Goals:
- **Memory and image size:**
  - ~2x less memory per small object;
  - the builtins' `length`/`name` tables become one shared structure;
  - 16-byte entries where tables remain.
- **Speed:** IC hits by one identity compare, cached property *additions* (constructors'
  `this.x = v`), and receiver own-miss proofs for prototype caches.
- **Stable contracts:** no change to elements: `get_element_fast`, `set_element_fast`
  and `array_length` (infra's 0080) keep their contracts. The `Heap` property API
  (`get_own_property`, `define_own_property`, `lookup_property`, …) keeps its signatures.

### 7.2 Layout

An object's `PROPS` field points to one of three things:
- nothing (null): no named properties;
- a **property value block** `PropValues` (new cell kind): `SHAPE: Ref`, `LEN: u32`
  (slots in use), then `VALUES: [Value]` with capacity. Shape mode;
- a **dictionary** `PropTable` (as today, but with 16-byte entries, below). Dictionary
  mode, marked by the object header flag `OBJ_DICTIONARY` (so the mode needs no load of
  the target cell's kind).

The `Object` cell stays 24 bytes. The prototype stays in the object, not in the shape:
- changing a prototype touches no shape;
- objects with the same keys share a shape whatever their prototype;
- 0062's prototype caches already check the prototype separately.

A **shape** (new cell kind `Shape`, 24 B) has:
- `PARENT: Ref`: the shape without the last property;
- `DESCS: Ref`: a `U32Array` of `[key atom, flags]` pairs (8 B per property) in insertion
  order;
- `COUNT: u32`: the number of properties = the slots used;
- `TRANSITIONS: Ref`: a `RefArray` of the child shapes, sorted by the key and flags of
  their last descriptor (part 2, D1831).

`DESCS` is shared along a chain:
- a child that extends the descriptor array's current owner appends in place, if there
  is capacity, and becomes the owner (V8's descriptor sharing);
- a second child of the same parent copies the prefix.

So a chain of n shapes costs O(n) descriptor bytes, not O(n²). A property's slot is its
descriptor index.

**Accessors:** an accessor's value slot holds an internal ref to an `AccessorPair` cell
(new kind: `GETTER`, `SETTER`). Its flags mark it as an accessor. This is what lets
dictionary entries shrink to 16 bytes: key u32, flags u32, value (a data value or the
pair). That is 0052 S2's first item.

**Sizes:**

| object | today | shapes |
| --- | ---: | ---: |
| `{a, b, c, d}` | 144 B | 72 B (object 24 + values 16 + 4 × 8) |
| a builtin function's `length`/`name` | 72 B table | 32 B values block + one shared shape |
| a 5-key JSON record | 240 B (capacity 8) | 80 B |

### 7.3 Transitions

Adding a key (or a key with non-default flags) to a shape-mode object goes from shape `S`
to the child for `(key, flags)`. The child is found in `S.TRANSITIONS`, or created and
added there.
- **Strong transitions (D1831; part 2 changed this from the first proposal):** a parent
  keeps its children alive, so shapes never die. Weak (ephemeron) transitions would make
  the tree depend on when collections ran: a recreated shape lands elsewhere in its
  parent's list, a dead owner leaves its descriptor array marked as used so the new shape
  copies instead of sharing, and any per-shape limit would count dead children. All of
  that changes the image and so the canonical hash (D1730), and could change whether an
  object becomes a dictionary. With strong transitions the tree is a function of the
  program's history alone.
- **Bounded growth instead:** at most 32 properties per shape (`SHAPE_MAX`), and a
  per-heap **shape budget** of 4096 shapes (`SHAPE_BUDGET`, counted in `Roots::SHAPES`,
  part of the image). Once the budget is spent, key sets without a shape go to dictionary
  mode; existing shapes are still used. A shape costs about 40 bytes (the cell, its slot
  in the parent's transitions, its share of the descriptors), so the tree stays below
  about 160 KB typically and 2.3 MB at worst (every shape copying a 32-property prefix).
- **Lookup:** a binary search over the children, which are sorted by key and flags. There
  is no limit on children per shape: the root has a child for every first key, and the
  first proposal's limit of 32 made every object a dictionary once 32 distinct first keys
  had been seen.
- **Bulk creation:** `new_object_from_entries` (JSON.parse, wire decode, object literals
  with a known count) walks the transitions once per key and allocates the values block
  at its exact size. JSON records with the same keys share one shape chain.
- **Values growth:** the block doubles (minimum 4) when a key is added through property
  definition, like tables today. `compact_for_image` shrinks blocks to their size.

### 7.4 Dictionary mode

Shapes stay small and shared. An object switches to dictionary mode (a 16-byte-entry
`PropTable` in insertion order, keeping enumeration order) when:
- it gets more than **32** properties;
- a key is deleted, unless it is the last one added (the object goes back to the parent
  shape) or the shape without it needs at most one new shape (part 2: the tree is walked
  again from the deleted key's parent and the values move down, so `delete t.x` on small
  records stays in shape mode without spending the budget on every subset of a map-like
  object's keys);
- an existing property changes its attributes (`defineProperty` on an existing key,
  freeze/seal);
- it gets an array-index key in the table (`OBJ_INDEXED_PROPS`);
- the shape budget is spent and its key set has no shape yet;
- it is a builtin namespace or prototype object of the realm (`InstallMethod`,
  `InstallValue` and `InstallAccessor` targets, and the global object). These are
  one-off objects, so a private table is smaller than a chain of unshared shapes. Their
  dictionaries also shrink with the 16-byte entries.

Dictionary mode is one-way, as in V8: objects never go back to shapes.

### 7.5 GC, determinism, snapshots

- **Ordinary cells:** shapes, descriptor arrays, transition arrays and values blocks are
  traced like any other cell; the root shape is a `Roots` field, created on the first
  shape-mode addition. Nothing is keyed by an offset, so no rehash is needed.
- **Deterministic trees (D1090, D1730):** the shape tree is built in program order and
  only grows (strong transitions, 7.3), so a collection only moves it. The canonical hash
  is the same whatever the collection points, the budget's overflow included (vm/tests.rs
  `shapes_do_not_depend_on_collection_points`).
- **Snapshots:** shapes are part of the image; the layout hash changes as for any layout
  change. The base image's shape tree is built by the prelude, so it is identical on
  every build of the same source (0032, 0072's native-vs-wasm check).
- **Out-of-memory (D1600/D1700):** a transition allocates a shape and maybe a descriptor
  array and a transition array, then the values block and an accessor pair. All
  allocations come before the commit (`PROPS`/`SHAPE` written last; the tree links a new
  shape only after its cells exist), so 0070's retry stays valid for `put_field`,
  `define_field`, `Set` and `CreateDataPropertyOrThrow`. A retry finds the shape the
  failed attempt created. Deleting a property can now allocate (a dictionary conversion),
  so `Heap::delete_own_property` returns a `Result`.

### 7.6 The JS reader (reader.ts)

`plain()` reads either mode, using `layout.generated.ts`:
- `OBJ_DICTIONARY` set: the table with 16-byte entries;
- otherwise: `PROPS` is a values block; its `SHAPE` gives `DESCS` and `COUNT`; entry i is
  `DESCS[2i]`, `DESCS[2i+1]` and `VALUES[i]`. Accessors are skipped, as today.

`array()` reads index keys from the table: arrays with sparse or attributed elements are
always in dictionary mode. `checkHeap` gets the new kinds. docs/wire.md's
reader section is updated; the wire format itself doesn't change.

### 7.7 Inline caches (D1620/D1621) on shapes

As built (parts 2 and 3, D1832). `IC_WORDS` went from 3 to 5; the slot layouts are
documented in `interp/run/ic.rs`, and the hit checks are two shared functions,
`cached_get` and `cached_put`, so the run loops have one implementation.
- **Own read/write (shape mode, part 2):** word 0 = `IC_OFFSET_SLOT | slot + 1` (kind 0),
  word 1 = the shape, word 2 = writable. A hit compares the object's shape and reads or
  writes `VALUES[slot]`.
- **Own read/write (dictionary mode):** unchanged, a verified table index. The global
  object and the lexical record are dictionaries, so the global caches don't change.
- **Additions:** `put_field` and `define_field` (which got a `Cache` immediate) cache
  the transition: kind `KIND_ADD`, the receiver's shape word before and the shape
  after. A hit writes the value, `LEN` and the new `SHAPE`, growing a full values block,
  and doesn't search anything. This is the constructor and object-literal case that 0062
  could not cache. A `put_field` addition is [[Set]], so it also depends on the
  prototype chain: the cache is filled only when no object on the chain has the key and
  none is a proxy or a typed array. The slot keeps the receiver's prototype (word 3) and
  the prototype epoch (word 4), and every object of the chain is marked `OBJ_IS_PROTO`,
  so a key added anywhere on it, or a prototype change, misses. `define_field` is
  CreateDataProperty and needs neither. Objects marked as prototypes don't take cached
  additions, since theirs must bump the epoch. Proxies are dictionaries, so no shape
  compare ever matches one.
- **Prototypes:** word 3 is the receiver's shape word. When the receiver's shape word
  matches it, the receiver has no own `key`, which replaces the per-hit
  `has_own_property`. Any other receiver (another shape, a dictionary) still looks it
  up, so polymorphic sites keep hitting. An array's `length` and typed arrays never use
  the proof.
- **Offsets and the GC:** shape offsets in a slot set `IC_OFFSET_SLOT`, so the collector
  empties them (D1621), and warm and cold caches still behave the same. The rest of
  D1620/D1621 stays.

### 7.8 What else of 0052 S2

- 16-byte entries: part of this design.
- Shared `length`/`name`: shapes.
- A slimmer template for privileged (prelude) functions: no `NAMES`, `LINES`, `SRC_*` or
  `IC` fields, so about 48 B instead of 80 B. Independent of shapes.
- The atom index at ~75% load: independent.

I'd do the last two as their own small parts at the end, since they don't depend on
shapes.

### 7.9 Plan, in parts (each merged separately, with numbers)

1. `AccessorPair` and 16-byte dictionary entries; reader update.
2. Shapes for ordinary objects: `Shape`/`PropValues` cells, transitions, dictionary
   fallbacks, bulk creation, namespaces and prototypes as dictionaries,
   `compact_for_image`, walk/verify, reader.
3. ICs on shapes: shape compare, transition caches, receiver-shape proofs.
4. Optional: slim privileged templates; atom index load factor.

**Expected:**
- 2–3x less memory for small objects;
- the base image shrinks by about 25 KB from parts 1–2 (about 6%), and parts 4 add the
  rest of S2's estimate (70–90 KB in total);
- faster property additions;
- unchanged or slightly faster accesses.

**Risk:** part 2 rewrites `heap/object.rs`'s storage layer (~1,500 lines). Every exotic
class (arrays, String wrappers, typed arrays, functions, arguments, proxies) keeps its
current special cases above the storage layer.

### 7.10 Open questions

- Values inline in the object (V8's in-object slack) would save the 16-byte block header
  and one load, but objects can't grow in place under a copying GC with identity by
  offset. I propose out-of-line blocks only.
- The 32/32 limits are guesses; part 2 measures them on the engine track and the pi
  bench.

### 7.11 Results of part 2 (0083, D1831)

Byte counts (deterministic; arena bytes are the same natively and in wasm, which the
native-vs-wasm check confirms):

| | main | shapes |
| --- | ---: | ---: |
| base image | 406,784 B | 397,864 B (−8.9 KB) |
| `{a,b,c,d}` literal, per object in the arena | 122.5 B | 82.5 B (−33%) |
| JSON.parse 5-key record | 178.5 B | 130.5 B (−27%) |
| class instance, 3 fields | 122.5 B | 82.5 B (−33%) |
| closure | 154.5 B | 114.5 B (−26%) |
| wasm module (`wasm` profile) | 1,485,742 B | 1,489,314 B (code +12.2 KB, data −8.6 KB) |

The per-object numbers are from the wasm build (100,000 objects kept in an array, after a
collection, `work/scratch/0083/wasm-mem.ts`). Part 1's 16-byte entries are in both
columns. Linear memory grows about 2.7× the arena bytes for these runs (the to-space and
the arena's doubling). That ratio is the collector's business (0087), not the layout's.

Base image: 501 values blocks (16.4 KB), 80 dictionaries (14.4 KB), 38 shapes (0.9 KB, plus
0.8 KB of descriptors and 0.7 KB of transition arrays).

Time (load 10–15, so only relative, interleaved): the engine track and pi-like are
unchanged within ±3% natively (LTO builds) and in wasm (node). Before the delete
re-walk and the shape-compare own caches, `props.js` was +22%: verified-index hits load
more in shape mode, and `delete t.x` converted every record to a dictionary.

The limits on realistic code (`shape_stats_report`): the engine track creates 0–6 shapes
per script, pi-like 16, and the difftest corpus 3 per case at the median, 59 at most.
No case reached 32 properties on a shape-mode object or the budget. Dictionaries made
while running come from index keys (arguments objects, String wrappers), a few attribute
changes and deletes.

### 7.12 Results of part 3 (0083, D1832)

Timing at load 7–15: only relative, interleaved, min of 7–15 runs; native = LTO builds.

| | native | wasm (node) |
| --- | ---: | ---: |
| object literals `{x, y}` + 3 assignments, 1M (`p-new2`) | −64% | −71% |
| constructors (2 classes, 3 fields), 1M (`p-ctor`) | −65% | −69% |
| `props.js` | −15% | −14% |
| `strings.js` | −7% | −12% |
| pi-like | −1% | −12% |
| the rest of the engine track | −4..−1% | −1..+2% |
| own writes only (`p-w`, 5M) | ±0% | +7..+13% |

The one regression is pure own-property writes in wasm, while the same code is
unchanged natively. Moving the define arm out of `fast_step` and reading the slot head
with one bounds check helped a little. It looks like V8 codegen around the larger fast
path; 0093's `run_hot` restructures that loop anyway. Memory: an IC slot is 20 bytes
instead of 12, and only for functions that ran.
