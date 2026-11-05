# #2: returning `Result`s in registers (design sketch, not started)

Deferred until 0224 lands (coordinator). Estimate −5 to −10 KB gzip; likely faster.

## The cost today

`Fault` is `Box<FaultKind>` (D1574), so `Result<(), Fault>` is one pointer, and so is
`Result<Obj, Fault>`'s error, but every result that carries a `Value` is 16 bytes:
`Value(u64)` has no invalid bit pattern (0 is +0.0), so the discriminant needs its own
word, and the wasm32 Rust ABI returns anything wider than one scalar through a
caller-allocated stack slot. Each call writes the slot, and each `?` loads the tag,
branches, and on the error path copies the box into the caller's own return slot.
`Step = Result<Option<Status>, Fault>` has the same shape (8 bytes, through memory).
The line table charges 53 KB raw of the 832 KB of code to core's result.rs (the `?`
branch and `from_residual` moves), 22 KB to option.rs.

## The change

1. **A niche in `Value`:** `Value(NonZeroU64)`, with a representation in which no value
   has the bits 0. For example, store `bits ^ K`, where K is a NaN-space pattern that no
   value uses (an unused tag in the TAG_PREFIX space). Tag tests compare against XORed
   constants (free); `as_f64`/`number` add one XOR. Then `Option<Value>` and
   `Result<Value, E>` for a zero-sized `E` are 8 bytes, a single i64 returned in a
   register. Images store values XORed too: their format changes (COMPAT_VERSION bump;
   djs is unreleased, D024).
2. **A zero-sized `Fault`:** the pending error (`FaultKind`: a thrown value, an error to
   create, a heap error) moves into the VM (`Heap::pending`, next to the arena), and
   `Fault` becomes the token "the VM holds an exception". Only one exception is ever
   pending, because a fault unwinds to a handler before anything else runs. Constructors
   take the heap: `self.throw(v)`, `self.error(kind, msg)` instead of `Fault::throw(v)`
   (77 sites). Heap methods that fail record their `HeapError` in the heap and return
   the token, so `From<HeapError> for Fault` and its `?` sites keep working unchanged,
   provided `HeapError` itself becomes the same token (the heap has `self` wherever it
   fails).
3. `Step` becomes `Result<Option<Status>, Fault>`, 1 byte, in a register.

What stays: `?` ergonomics, no `unsafe`, the same exception semantics. What changes:
every place that inspects a `Fault`'s kind (`is_out_of_memory`, catch-handler
dispatch, `fault_value`) reads `heap.pending` instead.

## Risks and measurement plan

- Hot paths everywhere (run_hot, step, IC, GC never), so this needs wmeasure on the 12
  workloads and the native benches; the XOR on f64 boxing is the only new instruction.
  Fewer stack round trips on every fallible call should more than pay for it.
- A stale `pending` after a caught error would be a bug class of its own: clear it where
  a handler takes the exception (`fault_value`), and add a debug assertion that it is
  empty on entry to a run.
- Prototype order: first (1) alone, measuring `Option<Value>` and the `Result<Value, _>`
  sites with `Fault` still boxed (no gain expected there; it measures the XOR cost),
  then (2) in one module (conv.rs + intrinsics) behind the token to measure bytes per
  site, then the whole change.

## Prototype measurements (2026-11-05, on 21f1740c: 428,507 gzip)

A scratch copy (work/scratch/0225/niche/, never merged):

- **Value niche:** `Value(NonZeroU64)` in register form `raw ^ TAG_PREFIX`; memory stays
  raw (zero-filled memory is still +0), so images do not change. Tag tests fold into
  constants; f64 boxing and unboxing and every arena load and store of a value gain an
  XOR, and a safe construction also needs a zero check (`NonZeroU64::new(..)`
  with a fallback).
- **Zero-sized Fault:** `Fault(Pend)`, with the `FaultKind` in a global cell (prototype
  only: `unsafe`, single-threaded). That measures the code shape a VM-held pending
  error would give.

| variant | module gzip | Δ |
| --- | ---: | ---: |
| baseline | 428,507 | |
| Value niche, checked construction | 433,069 | +4.6 KB |
| + zero-sized Fault (checked) | 432,457 | +4.0 KB |
| + zero-sized Fault, unchecked construction (`new_unchecked`, the lower bound) | 431,090 | +2.6 KB |
| zero-sized Fault alone | 428,318 | −0.2 KB |

Wasm instructions, the checked niche plus zero-sized Fault vs the baseline (n = 2):
tight +7.9%, prop1 +5.7%, arrays +2.9%, json +0.9%, calls +8.0%, pi-like +0.6%.

So the estimate was wrong. The extra XOR (and the zero check) on every value that
crosses the arena costs more code than the register returns save, and returning 16-byte
results through memory turns out to be cheap in gzip terms: the sequences are uniform.
The zero-sized `Fault` alone saves 0.2 KB, which doesn't pay for the API change (every
fault constructor needs the VM). **Recommendation: don't build #2.** What does remain
is the `Result` plumbing of the *error* side (`from_residual` moves), but that would need
a different error model (a flag in the VM and plain return values), which is a
redesign, not a size fix.
