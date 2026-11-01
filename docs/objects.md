# Property access: measurements and design (0062)

Status: design proposal, not implemented. Decisions D1620+ get recorded when parts land.

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
