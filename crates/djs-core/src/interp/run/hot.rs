//! The tier-0 loop (D1930): the interpreter's fast paths with its state in locals.
//!
//! `run_hot` loads `pc`, `sp`, `fuel` and the frame data from `Exec` into locals, takes
//! the arena as a local slice, and executes instructions until it meets one that needs
//! the general path (`step`): an instruction it does not handle, or a case of one that
//! needs it (a non-number operand, an IC miss, a callee that is not a plain closure, ...).
//! It writes `pc`, `sp` and `fuel` back once, when it returns; the frame fields are
//! written whenever a call or return changes the frame (the inline caches read
//! `Exec::template`).
//!
//! Nothing it does can collect or run user code: every collection, user call and
//! fallible operation is in `step`. The leaf heap operations it calls (inline-cache
//! hits, dense elements, an Array Iterator step) end the slice borrow first and re-take
//! it after, since one may grow the arena (`set_element_fast` appends) and set
//! `gc_pending`, which is re-read with it. The stack cell never moves here (only `ensure`
//! grows it, in `step`), and no cell moves (no collection).
//!
//! It is exactly equivalent to the general path: the same fuel per instruction, a fuel
//! check at every instruction boundary (it returns with fuel <= 0 and the run loop stops
//! there), the same arena writes, and the same safepoints (frame entry, return and
//! backward jumps: with a collection wanted, the instruction completes, then `run_hot`
//! runs `safepoint()` at that point). So suspension points, collections and heap images
//! do not change (vm/tests.rs `hot_loop_is_invisible`).

use super::ic::Hit;
use super::{Exec, fast};
use crate::bytecode::decode::arity;
use crate::bytecode::special;
use crate::bytecode::{OPS, Op};
use crate::heap::HeapError;
use crate::heap::cells::{FuncTemplate, Header, Object, Realm, Roots, ValueArray};
use crate::heap::layout::{arena_read, arena_write};
use crate::heap::{Atom, ObjectClass, atoms};
use crate::interp::conv::{self, FaultKind};
use crate::interp::frame::{self, Continuation, HEADER};
use crate::interp::realm::Intrinsic;
use crate::value::Value;

/// Fuel after which `run_hot` returns to the run loop at the next backward jump anyway: V8
/// tiers wasm code up only between calls (no OSR, 0081 F0), so a loop that never leaves
/// `run_hot` would stay in Liftoff code for a whole `vm_run`. Returning changes no state.
/// (A fuel mark, not a count of jumps: a counter is one more value that changes in the
/// loop, and the loop has `fuel` at hand, D3392.)
pub(crate) const HOT_FUEL: u32 = 1 << 17;

const IS_NUMBER: u16 = super::intrinsics::id("IsNumber");
const IS_OBJECT: u16 = super::intrinsics::id("IsObject");
const AS_NUMBER: u16 = super::intrinsics::id("AsNumber");
const AS_OBJ: u16 = super::intrinsics::id("AsObj");
const TO_OBJECT: u16 = super::intrinsics::id("ToObject");
const IS_CALLABLE: u16 = super::intrinsics::id("IsCallable");
const GET: u16 = super::intrinsics::id("Get");
const CREATE_DATA_PROPERTY_OR_THROW: u16 = super::intrinsics::id("CreateDataPropertyOrThrow");
const CALL: u16 = super::intrinsics::id("Call");
const ARRAY_CREATE: u16 = super::intrinsics::id("ArrayCreate");
const ARGUMENT_COUNT: u16 = super::intrinsics::id("ArgumentCount");

const GET_GLOBAL: u8 = Op::GetGlobal as u8;
const GET_GLOBAL_TYPEOF: u8 = Op::GetGlobalTypeof as u8;
const PUT_GLOBAL: u8 = Op::PutGlobal as u8;
const IN: u8 = Op::In as u8;
/// Not an opcode: what the `get_field` arm declines with when its cache slot points at an
/// accessor (`decline_getter`). A miss declines with the opcode and goes to `step`.
const GETTER: u8 = 255;
const _: () = assert!(crate::bytecode::OP_COUNT < GETTER as usize);
const TO_STRING: u8 = Op::ToString as u8;
const CONCAT: u8 = Op::Concat as u8;
const TYPEOF: u8 = Op::Typeof as u8;

/// The flags word of a `PushResult` frame (`fast_return`'s condition).
const PUSH_RESULT: i32 = Continuation::PushResult.to_flags();
const EACH: i32 = Continuation::Each.to_flags();

/// The intrinsics `Exec::pure_leaf` runs, by name. The hot loop calls it only for these,
/// so the others decline without a call (a test checks the list against `pure_leaf`).
pub(super) const PURE_LEAVES: &[&str] = &[
    "IsObject",
    "ConstructorPrototype",
    "CollectionCreate",
    "PromiseCreate",
    "ErrorCreate",
    "DateCreate",
    "NewBuiltinClosure",
    "ErrorStack",
    "IsPendingStack",
    "CaptureStack",
    "IsCallable",
    "IsConstructor",
    "IsArray",
    "IsString",
    "IsNumber",
    "IsSymbol",
    "IsBoolean",
    "AsObj",
    "AsFn",
    "AsStr",
    "AsNumber",
    "AsSym",
    "AsBool",
    "ToBoolean",
    "ToNumberPrimitive",
    "ToStringPrimitive",
    "ToObject",
    "ToIntegerOrInfinityNumber",
    "F64ToU32",
    "F64ToI32",
    "StrLength",
    "SameValue",
    "PrimitiveValue",
    "NumberToString",
    "NumberToFixed",
    "NumberToExponential",
    "NumberToPrecision",
    "StringParseFloat",
    "StringParseInt",
    "StrCharCodeAt",
    "StrCodePointAt",
    "StrSubstring",
    "StrIndexOf",
    "StrLastIndexOf",
    "StrToCase",
    "StrTrim",
    "StrIsWellFormed",
    "StrToWellFormed",
    "StrRepeat",
    "StrNormalize",
    "F64Abs",
    "F64Acos",
    "F64Acosh",
    "F64Asin",
    "F64Asinh",
    "F64Atan",
    "F64Atanh",
    "F64Atan2",
    "F64Cbrt",
    "F64Hypot",
    "F64Ceil",
    "F64Clz32",
    "F64Cos",
    "F64Cosh",
    "F64Exp",
    "F64Expm1",
    "F64Floor",
    "F64Fround",
    "F64F16round",
    "F64Imul",
    "F64Log",
    "F64Log1p",
    "F64Log10",
    "F64Log2",
    "F64Round",
    "F64Sign",
    "F64Sin",
    "F64Sinh",
    "F64Sqrt",
    "F64Tan",
    "F64Tanh",
    "F64Trunc",
];

const PURE_WORDS: usize = crate::bytecode::intrinsics::INTRINSICS.len().div_ceil(64);

/// [`PURE_LEAVES`] as a bit per intrinsic id.
static PURE: [u64; PURE_WORDS] = {
    let mut bits = [0u64; PURE_WORDS];
    let mut k = 0;
    while k < PURE_LEAVES.len() {
        let i = super::intrinsics::id(PURE_LEAVES[k]) as usize;
        bits[i / 64] |= 1 << (i % 64);
        k += 1;
    }
    bits
};

pub(super) fn is_pure_leaf(id: u16) -> bool {
    PURE.get(id as usize / 64)
        .is_some_and(|w| w >> (id % 64) & 1 != 0)
}

/// An opcode's encoded size and its fuel, as constants. An arm of several opcodes that
/// asks the table about an opcode it found at run time makes the loop index the table
/// and load both (about 15 machine instructions in V8's code, D3392): such arms name one
/// opcode, and the assertions below check that the others agree.
const fn size_of(op: Op) -> u32 {
    OPS[op as usize].size as u32
}

const fn fuel_of(op: Op) -> i64 {
    OPS[op as usize].cost.fuel() as i64
}

/// Whether `a` and `b` advance and cost the same.
const fn same_step(a: Op, b: Op) -> bool {
    size_of(a) == size_of(b) && fuel_of(a) == fuel_of(b)
}

const _: () = assert!(
    same_step(Op::Inc, Op::Dec)
        && same_step(Op::IncLoc, Op::DecLoc)
        && same_step(Op::StrictEq, Op::StrictNe)
        && same_step(Op::GetField, Op::GetMethod)
        && same_step(Op::JumpIfTrue, Op::JumpIfFalse)
        && same_step(Op::Lt, Op::Le)
        && same_step(Op::Lt, Op::Gt)
        && same_step(Op::Lt, Op::Ge)
        && same_step(Op::PutField, Op::DefineField)
        && fuel_of(Op::Call) == fuel_of(Op::Call0)
        && fuel_of(Op::CallIntrinsic) == fuel_of(Op::CallIntrinsic8)
        && fuel_of(Op::Return) == fuel_of(Op::ReturnUndef)
);
// The closure-variable arm's opcodes all step as `get_cap`.
const _: () = {
    let ops = [
        Op::GetCapChk,
        Op::PutCap,
        Op::PutCapChk,
        Op::SetCap,
        Op::SetCapChk,
        Op::GetBox,
        Op::GetBoxChk,
        Op::PutBox,
        Op::PutBoxChk,
        Op::SetBox,
        Op::SetBoxChk,
    ];
    let mut i = 0;
    while i < ops.len() {
        assert!(same_step(ops[i], Op::GetCap));
        i += 1;
    }
};

#[cold]
#[inline(never)]
fn out_of_bounds() -> ! {
    panic!("arena access out of bounds")
}

// The single-field accessors go through the arena's own (heap/layout/access.rs): one
// compare, or none in the shipped wasm modules (`unsafe-heap`, D3100). Kept checked
// here, the slot loads and stores of every instruction were ~5% of a tight loop's
// instructions. The block writes below keep one slice check per block.

/// The Value at byte offset `off`.
#[inline(always)]
pub(super) fn ld(b: &[u8], off: u32) -> Value {
    Value::from_raw(u64::from_le_bytes(arena_read(b, off as usize)))
}

#[inline(always)]
fn st(b: &mut [u8], off: u32, v: Value) {
    arena_write(b, off as usize, v.to_bits().to_le_bytes())
}

#[inline(always)]
fn ld32(b: &[u8], off: u32) -> u32 {
    u32::from_le_bytes(arena_read(b, off as usize))
}

/// `n` Values at `off` set to `v`, with one range check.
#[inline(always)]
fn fill(b: &mut [u8], off: u32, n: u32, v: Value) {
    let o = off as usize;
    let Some(s) = b.get_mut(o..o + 8 * n as usize) else {
        out_of_bounds()
    };
    let bits = v.to_bits().to_le_bytes();
    for c in s.as_chunks_mut::<8>().0 {
        *c = bits;
    }
}

/// Writes a new frame at stack index `base` (D1931): the argument padding from `argc` to
/// `n`, the 6-word header at `hp`, and `locals` undefined locals after it, each as one
/// checked block. `vals` is the stack's value base.
#[inline(always)]
pub(super) fn write_frame(
    b: &mut [u8],
    vals: u32,
    (base, argc, n, hp, locals): (u32, u32, u32, u32, u32),
    header: [Value; HEADER as usize],
) {
    let slot = |i: u32| vals + 8 * i;
    fill(b, slot(base + 2 + argc), n - argc, Value::UNDEFINED);
    let o = slot(hp) as usize;
    let Some(h) = b.get_mut(o..o + 8 * HEADER as usize) else {
        out_of_bounds()
    };
    for (c, v) in h.as_chunks_mut::<8>().0.iter_mut().zip(header) {
        *c = v.to_bits().to_le_bytes();
    }
    fill(b, slot(hp + HEADER), locals, Value::UNDEFINED);
}

/// `vs` at `off`, with one range check.
#[inline(always)]
pub(super) fn write_values(b: &mut [u8], off: u32, vs: &[Value]) {
    let o = off as usize;
    let Some(s) = b.get_mut(o..o + 8 * vs.len()) else {
        out_of_bounds()
    };
    for (c, v) in s.as_chunks_mut::<8>().0.iter_mut().zip(vs) {
        *c = v.to_bits().to_le_bytes();
    }
}

/// The fields of a function template a call reads (PARAMS .. FLAGS, in one read).
pub(super) struct TemplateHead {
    pub flags: u32,
    pub params: u32,
    pub locals: u32,
    pub stack: u32,
    pub code_len: u32,
    pub consts_ref: u32,
}

#[inline(always)]
pub(super) fn template_head(b: &[u8], t: u32) -> TemplateHead {
    const W0: u32 = FuncTemplate::PARAMS.offset;
    // The fields must lie in the window (a layout change shows up here).
    const fn inside(o: u32, n: u32) -> bool {
        o >= W0 && o + n <= W0 + 32
    }
    const _: () = assert!(
        inside(FuncTemplate::FLAGS.offset, 2)
            && inside(FuncTemplate::PARAMS.offset, 2)
            && inside(FuncTemplate::LOCALS.offset, 2)
            && inside(FuncTemplate::STACK.offset, 2)
            && inside(FuncTemplate::CODE_LEN.offset, 4)
            && inside(FuncTemplate::CONSTS.offset, 4)
    );
    let tw: [u8; 32] = window(b, t + W0);
    let tf = |f: u32| (f - W0) as usize;
    TemplateHead {
        flags: u16_at(&tw, tf(FuncTemplate::FLAGS.offset)),
        params: u16_at(&tw, tf(FuncTemplate::PARAMS.offset)),
        locals: u16_at(&tw, tf(FuncTemplate::LOCALS.offset)),
        stack: u16_at(&tw, tf(FuncTemplate::STACK.offset)),
        code_len: u32_at(&tw, tf(FuncTemplate::CODE_LEN.offset)),
        consts_ref: u32_at(&tw, tf(FuncTemplate::CONSTS.offset)),
    }
}

/// `N` bytes at `off`, with one range check.
#[inline(always)]
fn window<const N: usize>(b: &[u8], off: u32) -> [u8; N] {
    arena_read(b, off as usize)
}

fn u16_at(w: &[u8], i: usize) -> u32 {
    u16::from_le_bytes([w[i], w[i + 1]]) as u32
}

fn u32_at(w: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([w[i], w[i + 1], w[i + 2], w[i + 3]])
}

/// The instruction at `ip` and the 7 bytes after it. The frame's code must be
/// [`fetchable`]: the read is the arena's (checked natively and in tests, a plain load
/// in the shipped modules, D3100).
#[inline(always)]
fn word(b: &[u8], ip: u32) -> u64 {
    u64::from_le_bytes(arena_read(b, ip as usize))
}

/// Whether every instruction of the code ending at `end` can be fetched as a [`word`]:
/// the last one may be a single byte, so 7 more bytes must exist. False only for code
/// within the last bytes of the arena (the general path runs it).
#[inline(always)]
fn fetchable(b: &[u8], end: u32) -> bool {
    end as u64 + 7 <= b.len() as u64
}

/// The arena offset of slot `i` of the object (or box) at `cell` with `Object::SLOTS`'s
/// check that `i` is a slot (as `Heap::object_slot`).
#[inline(always)]
fn object_slot(b: &[u8], cell: u32, i: u32) -> u32 {
    let size = ld32(b, cell + Header::SIZE.offset);
    // `i < tail_capacity(size)` without its division (a call in wasm, 0107): slot `i`
    // ends inside the cell.
    assert!(
        Object::SLOTS.at(cell, i) + 8 <= cell + size,
        "slot {i} out of range"
    );
    Object::SLOTS.at(cell, i)
}

/// `a.length` of an Array (its slot 0) and of a string primitive (0086), read from the
/// loop's slice; `None` for anything else (and a length above the int32 range).
#[inline(always)]
fn hot_length(b: &[u8], o: Value) -> Option<Value> {
    if let Some(a) = o.as_object() {
        return (ld32(b, a.offset() + Object::CLASS.offset) == ObjectClass::Array as u32)
            .then(|| ld(b, object_slot(b, a.offset(), 0)));
    }
    let s = o.as_string()?;
    i32::try_from(ld32(b, s.0 + crate::heap::cells::Str8::LEN.offset))
        .ok()
        .map(Value::int)
}

/// `o[key]` for an int key and a dense, non-hole element (`Heap::get_element_fast`
/// without its typed-array branch), read from the loop's slice.
#[inline(always)]
fn hot_elem(b: &[u8], o: Value, key: Value) -> Option<Value> {
    let obj = o.as_object()?;
    let i = key.as_int().and_then(|i| u32::try_from(i).ok())?;
    // (A closure's `ELEMENTS` is its template, which reads as an empty array, D2448.)
    let el = ld32(b, obj.offset() + Object::ELEMENTS.offset);
    if el == 0 || i >= ld32(b, el + ValueArray::LEN.offset) {
        return None;
    }
    let v = ld(b, ValueArray::VALUES.at(el, i));
    (!v.is_hole()).then_some(v)
}

/// Whether the strings at `x` and `y` are equal, if both are flat one-byte cells
/// (`Heap::str_eq` for those): the lengths, then eight bytes at a time. `None` for a
/// two-byte string, a slice or a rope, and for equal lengths past
/// [`crate::heap::work::FREE_COMPARE`]: the heap's comparison charges for those (0289).
#[inline(never)]
fn hot_str_eq(b: &[u8], x: u32, y: u32) -> Option<bool> {
    use crate::heap::CellKind;
    use crate::heap::cells::Str8;
    let kind = |s: u32| arena_read::<1>(b, (s + Header::KIND.offset) as usize)[0];
    if kind(x) != CellKind::Str8 as u8 || kind(y) != CellKind::Str8 as u8 {
        return None;
    }
    let n = ld32(b, x + Str8::LEN.offset);
    if n != ld32(b, y + Str8::LEN.offset) {
        return Some(false);
    }
    if n > crate::heap::work::FREE_COMPARE {
        return None;
    }
    // (A cell's size is a multiple of 8: the last word may reach into its padding.)
    let word = |s: u32, i: u32| u64::from_le_bytes(arena_read(b, (Str8::DATA.at(s, i)) as usize));
    let mut i = 0;
    while i + 8 <= n {
        if word(x, i) != word(y, i) {
            return Some(false);
        }
        i += 8;
    }
    Some(i == n || (word(x, i) ^ word(y, i)) << (64 - 8 * (n - i)) == 0)
}

/// An array index key (as `intrinsics::index_key`).
fn index_key(k: Value) -> Option<u32> {
    if let Some(i) = k.as_int() {
        return u32::try_from(i).ok();
    }
    k.as_number()
        .filter(|&n| (0.0..2147483648.0).contains(&n) && (n as u32) as f64 == n)
        .map(|n| n as u32)
}

/// Why the inner loop stopped.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Exit {
    /// The next instruction (this opcode) needs the general path.
    Decline(u8),
    /// The frame has no bytecode left (a native frame), or a `@fast` builtin declined
    /// its call: the general path. (Not `Decline(0)`: that payload made LLVM set the
    /// opcode's local at the loop head, two more instructions per instruction in V8's
    /// baseline code, D2443.)
    General,
    /// A safepoint completed with a collection wanted: run it now.
    Safepoint,
    /// Fuel ran out, or a backward jump after `hot_fuel` fuel: return to the run loop
    /// (V8 tier-up).
    Budget,
    /// `%EachDense`'s callback returned this value (0137).
    Each(Value),
}

impl Exec<'_> {
    /// A `put_field` or `define_field` cache hit (ic.rs's shared hit checks, 0083): true
    /// when `v` was stored into `o`. A cached addition may allocate (never collect), like
    /// `set_element_fast`. Through the heap, not the loop's slice: a store is the
    /// heap's to make (its write gate, and whatever a collector needs to hear of).
    #[inline(never)]
    fn field_store(&mut self, op: u8, key: Atom, ic: u32, o: Value, v: Value) -> bool {
        let Some(obj) = o.as_object() else {
            return false;
        };
        if op == fast::DEFINE_FIELD {
            self.ic_define(obj, ic, v)
        } else {
            self.ic_put(obj, key, ic, v)
        }
    }

    /// Every `get_field`/`get_method` cache hit (0083), out of line: the loop asks it
    /// for those that `hot_length` and `ic::hot_get` leave (a dictionary-mode receiver, a
    /// receiver whose shape is not the remembered one, the rarer primitives, an absent
    /// key, an accessor: D3470), and debug builds compare the slice's hits with it. An
    /// accessor's getter is left in `hot_getter` for `decline_getter`; with `room` (the
    /// stack has the two slots `call_accessor` would ask for) a getter with a `@fast`
    /// entry runs here instead and its result comes back as `Hit::Fast` (D3530).
    #[inline(never)]
    fn field_slow(&mut self, pc: u32, o: Value, room: bool) -> Option<Hit> {
        // The instruction at `pc`, read again: the loop then keeps its key and cache
        // index only up to its own slice read (D3550).
        let w: u64 = self.heap.arena().read(self.code + pc);
        let (op, key, ic) = (
            w as u8,
            Atom::from_id((w >> 8) as u32),
            (w >> 40) as u16 as u32,
        );
        // (`hot_length` answered these in a release build.)
        #[cfg(debug_assertions)]
        if key == atoms::LENGTH && op == fast::GET_FIELD {
            // `a.length` of an Array (its slot 0) and of a string primitive (0086).
            match (o.as_object(), o.as_string()) {
                (Some(a), _) if self.heap.object_class(a) == ObjectClass::Array => {
                    return Some(Hit::Value(self.heap.object_slot(a, 0)));
                }
                (_, Some(s)) => {
                    let n = i32::try_from(self.heap.str_len(s)).ok()?;
                    return Some(Hit::Value(Value::int(n)));
                }
                _ => {}
            }
        }
        let hit = self.ic_get(o, key, ic);
        if let Some(Hit::Getter(getter)) = hit {
            // (`get_method` of an accessor is `step`'s.)
            if op != fast::GET_FIELD {
                return None;
            }
            if room && let Some(v) = self.fast_getter(getter, o) {
                return Some(Hit::Fast(v));
            }
            self.hot_getter = getter;
        }
        hit
    }

    /// `fast_elem` on explicit operands, out of line: `o[key]`, or `o[key] = v`
    /// (`Some(undefined)`). The loop reads dense elements itself (`hot_elem`) and stores
    /// with an index key through `Heap::set_element_fast`; it asks here for the rest: a
    /// typed array's element, and a string key (`counts[word]`, D3402), which the heap's string-key helpers answer
    /// without running code or allocating: a read gives the data property on `o`'s
    /// chain, or undefined when nothing on it has the key; a store only replaces the
    /// value of an own writable data property. `None` (an accessor, a proxy, an
    /// addition, a hole, another kind of key) is the general path's.
    #[inline(never)]
    fn elem_slow(&mut self, put: bool, o: Value, key: Value, v: Value) -> Option<Value> {
        let obj = o.as_object()?;
        let Some(i) = key.as_int().and_then(|i| u32::try_from(i).ok()) else {
            let s = key.as_string()?;
            if put {
                return self
                    .heap
                    .string_key_set(obj, s, v)
                    .then_some(Value::UNDEFINED);
            }
            return self.heap.string_key_get(obj, s);
        };
        // (The loop makes the stores with an index key itself.)
        if put {
            return None;
        }
        self.heap.get_element_fast(obj, i)
    }

    /// `typeof v` for the loop, as `step` computes it: the string of the type's atom
    /// (`None` if it had to be made and the heap is full).
    #[inline(never)]
    fn typeof_hit(&mut self, v: Value) -> Option<Value> {
        let atom = conv::typeof_atom(self.heap, v);
        self.heap.atom_string(atom).ok().map(Value::string)
    }

    /// A declined `<`, `<=`, `>` or `>=` of two strings (`decline_fast`, D3472): the
    /// code unit order, as `Exec::compare` asks `conv::less_than`. False, with nothing
    /// changed, for other operands.
    #[inline(never)]
    fn decline_compare(&mut self, op: u8) -> bool {
        let (a, c) = (self.get(self.sp - 2), self.get(self.sp - 1));
        let (Some(x), Some(y)) = (a.as_string(), c.as_string()) else {
            return false;
        };
        use core::cmp::Ordering::{Greater, Less};
        // (A failed flattening: `step` compares again and throws.)
        let Ok(ord) = self.heap.str_cmp(x, y) else {
            return false;
        };
        let r = match op {
            fast::LT => ord == Less,
            fast::LE => ord != Greater,
            fast::GT => ord == Greater,
            _ => ord != Less,
        };
        self.sp -= 1;
        self.set(self.sp - 1, Value::bool(r));
        // (The four cost and advance the same: the table's check.)
        let info = &OPS[Op::Lt as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.pc += info.size as u32;
        true
    }

    /// A declined `to_string` or `concat` of a template literal (`Exec::concat_op`), in
    /// `decline_fast`. False, with nothing changed, when an operand is an object
    /// (ToPrimitive runs user code) or when it fails (a symbol, the heap cap): `step`
    /// runs it then, throws or retries.
    fn decline_concat(&mut self, op: u8) -> bool {
        let concat = op == CONCAT;
        let n = if concat { self.imm_u16(1) } else { 1 };
        if (0..n).any(|i| self.get(self.sp - n + i).as_object().is_some())
            || self.concat_op(concat).is_err()
        {
            return false;
        }
        let info = &OPS[op as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.pc += info.size as u32;
        true
    }

    /// A declined `get_field` whose cache slot points at an accessor, own or on the prototype
    /// chain (`ic::Hit::Getter`, D3470): calls the getter as `step` would
    /// (`call_accessor`), without `step`'s lookup. False, with nothing changed, for
    /// anything else and when the call fails (`step` runs the instruction then and
    /// throws).
    #[inline(never)]
    fn decline_getter(&mut self) -> bool {
        // (`field_slow` found it for this instruction just now.)
        let (getter, o) = (self.hot_getter, self.top(0));
        // A closure (bytecode, `@fast` or native): `call` then pushes a frame or runs
        // the builtin, and never stops the run.
        if !getter
            .as_object()
            .is_some_and(|f| self.is_class(f, ObjectClass::Function))
        {
            return false;
        }
        #[cfg(debug_assertions)]
        {
            // The slot's answer is the lookup's.
            use crate::heap::{Lookup, Property};
            let start = o
                .as_object()
                .or_else(|| crate::interp::realm::primitive_proto(self.heap, o));
            let found = start.map(|s| self.heap.lookup_property(s, self.imm_atom()));
            assert!(
                matches!(found, Some(Lookup::Found(_, Property::Accessor { getter: g, .. })) if g == getter),
                "the slot's getter differs from the lookup's"
            );
        }
        let (pc, sp, fuel) = (self.pc, self.sp, self.fuel);
        let info = &OPS[Op::GetField as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.pc += info.size as u32;
        self.sp -= 1;
        match self.call_accessor(getter, o, None) {
            Ok(None) => true,
            r => {
                debug_assert!(r.is_err(), "a getter stopped the run");
                (self.pc, self.sp, self.fuel) = (pc, sp, fuel);
                false
            }
        }
    }

    /// A declined `in` with a string key and an object (`decline_fast`): the heap's
    /// answer where no proxy is on the chain (D3402), else false with nothing changed.
    fn decline_in(&mut self) -> bool {
        let (k, o) = (self.get(self.sp - 2), self.get(self.sp - 1));
        let (Some(s), Some(obj)) = (k.as_string(), o.as_object()) else {
            return false;
        };
        let Some(r) = self.heap.string_key_has(obj, s) else {
            return false;
        };
        self.sp -= 1;
        self.set(self.sp - 1, Value::bool(r));
        let info = &OPS[Op::In as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.pc += info.size as u32;
        true
    }

    /// `fast_intrinsic`'s leaf cases (not `%Call`): the result, or `None`.
    #[inline(always)]
    fn intrinsic_hit(&mut self, id: u16, argc: u32, a: [Value; 3]) -> Option<Value> {
        Some(match (id, argc) {
            (IS_NUMBER, 1) => Value::bool(a[0].is_number()),
            (IS_OBJECT, 1) => Value::bool(a[0].as_object().is_some()),
            (AS_NUMBER, 1) if a[0].is_number() => a[0],
            (AS_OBJ, 1) | (TO_OBJECT, 1) if a[0].as_object().is_some() => a[0],
            // The callback builtins' checks (0137).
            (IS_CALLABLE, 1) => Value::bool(self.is_callable(a[0])),
            // Reduce's optional initialValue and its box (0162).
            (ARGUMENT_COUNT, 0) => Value::int(self.header_int(frame::ARGC) as i32),
            (ARRAY_CREATE, 1) if a[0] == Value::int(0) => {
                let proto = crate::interp::realm::intrinsic_obj(self.heap, Intrinsic::ArrayProto);
                Value::object(self.heap.new_array(Some(proto), 0).ok()?)
            }
            (GET, 2) => {
                let o = a[0]
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::Array)?;
                self.heap.get_element_fast(o, index_key(a[1])?)?
            }
            (CREATE_DATA_PROPERTY_OR_THROW, 3) => {
                let o = a[0]
                    .as_object()
                    .filter(|&o| self.heap.object_class(o) == ObjectClass::Array)?;
                let i = index_key(a[1])?;
                // Only a store that cannot fail or allocate: an existing dense element.
                self.heap.get_element_fast(o, i)?;
                if !matches!(self.heap.set_element_fast(o, i, a[2]), Ok(true)) {
                    return None;
                }
                Value::UNDEFINED
            }
            _ if is_pure_leaf(id) => return self.pure_leaf_hit(id, argc, a),
            _ => return None,
        })
    }

    /// `pure_leaf` for the hot loop: a fault declines (`call_intrinsic` runs the leaf
    /// again and throws), except a host refusal, which stops the run (D1690).
    #[inline(never)]
    fn pure_leaf_hit(&mut self, id: u16, argc: u32, a: [Value; 3]) -> Option<Value> {
        if argc > 3 {
            return None;
        }
        let w = self.heap.work_mark();
        // (Declined: `call_intrinsic` does the work again and pays for it.)
        match self.pure_leaf(id, a) {
            Ok(Some(v)) => Some(v),
            Ok(None) => {
                self.heap.work_reset(w);
                None
            }
            Err(f) => {
                self.heap.work_reset(w);
                if matches!(&*f.0, FaultKind::Heap(HeapError::HostOutOfMemory)) {
                    self.host_out_of_memory();
                }
                None
            }
        }
    }

    /// What the loop declined, done here instead of through `step` where that is cheap:
    /// a call of a `@fast` builtin (D1850), `+` of a string and a primitive whose
    /// ToString runs no user code and cannot throw (`"id " + i`, 0231), `in` with a
    /// string key (`decline_in`), a template literal's `to_string` and `concat`
    /// (`decline_concat`), or an
    /// instruction of the construct path (`decline_construct`, 0236). False, with
    /// nothing changed, for anything else or at the heap cap (`step` runs it then and
    /// retries, D1700). One out-of-line call: changes to the loop's own code, even a
    /// `continue` here, slowed plain loops and calls by 4-6% in V8 (0231).
    #[inline(never)]
    fn decline_fast(&mut self, op: u8) -> bool {
        if fast::is_call(op) {
            return self.pc < self.code_len && self.fast_leaf_call(op);
        }
        if op != fast::ADD {
            if op == IN {
                return self.decline_in();
            }
            if matches!(op, TO_STRING | CONCAT) {
                return self.decline_concat(op);
            }
            // (These two out of line: inside, they cost a `@fast` call, which runs in
            // this function, 8 machine instructions.)
            if op == GETTER {
                return self.decline_getter();
            }
            if matches!(op, fast::LT | fast::LE | fast::GT | fast::GE) {
                return self.decline_compare(op);
            }
            return super::construct::takes(op) && self.decline_construct(op);
        }
        use crate::value::Kind;
        let (a, c) = (self.get(self.sp - 2), self.get(self.sp - 1));
        if a.as_string().is_none() && c.as_string().is_none() {
            return false;
        }
        let part = |ex: &mut Self, v: Value| match v.as_string() {
            // (A string needs no conversion; asking for one cost 150 instructions.)
            Some(s) => Some(s),
            None if matches!(v.kind(), Kind::Object | Kind::Symbol) => None,
            None => conv::to_string(ex.heap, v).ok(),
        };
        let Some(s) = part(self, a)
            .and_then(|x| Some((x, part(self, c)?)))
            .and_then(|(x, y)| self.heap.concat(x, y).ok())
        else {
            return false;
        };
        self.sp -= 1;
        self.set(self.sp - 1, Value::string(s));
        let info = &OPS[Op::Add as usize];
        self.fuel -= info.cost.fuel() as i64;
        self.pc += info.size as u32;
        true
    }

    /// Runs instructions from `pc`, `step` included, until fuel runs out (`None`, the run
    /// loop stops there), the run stops (`Some`), or `hot_fuel` fuel went by and a
    /// backward jump or a general-path instruction is reached (`None`, for V8's tier-up).
    ///
    /// The loop does not check `pc` against the code's length per instruction (D3390).
    /// It relies on what the stack checker enforces for every template
    /// (`alloc_template_in`, the only place that makes one: the compiler's functions,
    /// lazily compiled ones and the builtins alike; images hold templates made that
    /// way): execution cannot fall off the end of the code, and every jump lands on an
    /// instruction inside it. Whether the frame has code at all (a native frame has
    /// none) and whether its last instruction can be fetched as a word is checked where
    /// the frame changes: on entry, in the call arm and in the return arm. Debug builds
    /// assert `pc < code_len` at every fetch.
    #[inline(never)]
    pub(super) fn run_hot(&mut self) -> Option<super::Status> {
        // The fuel at which a backward jump returns to the run loop (V8's tier-up).
        let floor = self.fuel - self.opts.hot_fuel as i64;
        let slow_call_flags = self.slow_call_flags;
        // The frame data, re-read after a collection (and kept current by calls and
        // returns here).
        let (mut code, mut end, mut bp, mut hp, mut lp): (u32, u32, u32, u32, u32);
        let (mut vals, mut consts, mut cap): (u32, u32, u32);
        macro_rules! load_frame {
            () => {
                code = self.code;
                end = code + self.code_len;
                (bp, hp, lp) = (self.bp, self.hp, self.lp);
                vals = ValueArray::VALUES.at(self.stack, 0);
                consts = self.consts;
                cap = self.heap.value_array_capacity(self.stack);
            };
        }
        load_frame!();
        loop {
            let mut ip = code + self.pc;
            let mut sp = self.sp;
            let mut fuel = self.fuel;
            let exit = 'run: {
                // Fuel the leaves charged and the next safepoint takes (0289): until
                // then the run does not stop for lack of fuel.
                let mut owed = self.heap.work_owed();
                // (A safepoint is wanted for a collection, or to take that fuel.)
                let mut collect = self.opts.gc_stress || self.heap.gc_pending() || owed;
                let mut b: &mut [u8] = self.heap.arena_mut().hot_bytes();
                // Ends the slice for a leaf heap call (`$e`), then takes it back.
                macro_rules! heap_call {
                    ($e:expr) => {{
                        let r = $e;
                        // The fuel of the call's work is taken at the next safepoint
                        // (0289), as the general path does.
                        owed = self.heap.work_owed();
                        collect = self.opts.gc_stress || self.heap.gc_pending() || owed;
                        b = self.heap.arena_mut().hot_bytes();
                        r
                    }};
                }
                macro_rules! slot {
                    ($i:expr) => {
                        vals + 8 * ($i)
                    };
                }
                // A native frame (no code), or code within the arena's last bytes: the
                // general path.
                if ip >= end || !fetchable(b, end) {
                    break 'run Exit::General;
                }
                loop {
                    if fuel <= 0 && !owed {
                        break Exit::Budget;
                    }
                    debug_assert!(ip < end, "pc past the end of the code");
                    let w = word(b, ip);
                    let sp0 = sp;
                    let mut op = w as u8;
                    // `heap_call!` for the arms of several opcodes: the opcode is read
                    // again after the call. Kept across it, V8 spills it where it is
                    // defined, for every instruction (D2441); single-opcode arms name
                    // their opcode instead.
                    macro_rules! heap_call_op {
                        ($e:expr) => {{
                            let r = heap_call!($e);
                            op = b[ip as usize];
                            r
                        }};
                    }
                    // The immediates, extracted in the arms that use them (D2441).
                    macro_rules! imm16 {
                        () => {
                            (w >> 8) as u16 as u32
                        };
                    }
                    macro_rules! imm32 {
                        () => {
                            (w >> 8) as u32
                        };
                    }
                    // Charges `op` (constants per arm) and advances past it.
                    macro_rules! next {
                        ($op:expr) => {{
                            let info = &OPS[$op as usize];
                            debug_assert_eq!(
                                sp + info.pop as u32,
                                sp0 + info.push as u32,
                                "hot {} stack effect disagrees with the table",
                                crate::bytecode::OP_NAMES[$op as usize]
                            );
                            fuel -= info.cost.fuel() as i64;
                            ip += info.size as u32;
                        }};
                        // `$op`, found at run time, steps as the constant `$like` does.
                        ($op:expr, like $like:expr) => {{
                            debug_assert!({
                                let info = &OPS[$op as usize];
                                sp + info.pop as u32 == sp0 + info.push as u32
                                    && info.size as u32 == size_of($like)
                                    && info.cost.fuel() as i64 == fuel_of($like)
                            });
                            fuel -= const { fuel_of($like) };
                            ip += const { size_of($like) };
                        }};
                    }
                    macro_rules! top {
                        ($k:expr) => {
                            ld(b, slot!(sp - 1 - $k))
                        };
                    }
                    macro_rules! push {
                        ($v:expr) => {{
                            let v: Value = $v;
                            st(b, slot!(sp), v);
                            sp += 1;
                        }};
                    }
                    // A jump to `ip + size + offset` charged as `$op`; a backward one is a
                    // safepoint and counts against the budget.
                    macro_rules! jump {
                        ($op:expr, $taken:expr) => {
                            jump!($op, $taken, imm32!() as i32)
                        };
                        ($op:expr, $taken:expr, $offset:expr) => {{
                            let info = &OPS[$op as usize];
                            fuel -= info.cost.fuel() as i64;
                            let next = ip + info.size as u32;
                            if !$taken {
                                ip = next;
                            } else {
                                let offset: i32 = $offset;
                                ip = (next as i64 + offset as i64) as u32;
                                if ip < next {
                                    if collect {
                                        break Exit::Safepoint;
                                    }
                                    if fuel < floor {
                                        break Exit::Budget;
                                    }
                                }
                            }
                        }};
                    }
                    // A binary op: `$int` on two int32s (`None`: try `$float`), `$float` on two
                    // Numbers (`None`: the general path).
                    macro_rules! binary {
                        ($op:expr, $int:expr, $float:expr) => {{
                            let (a, c) = (top!(1), top!(0));
                            let int: Option<Value> = match (a.as_int(), c.as_int()) {
                                (Some(x), Some(y)) => ($int)(x, y),
                                _ => None,
                            };
                            let v = match int {
                                Some(v) => v,
                                None => match (a.as_number(), c.as_number()) {
                                    (Some(x), Some(y)) => match ($float)(x, y) {
                                        Some(v) => v,
                                        None => break Exit::Decline(op),
                                    },
                                    _ => break Exit::Decline(op),
                                },
                            };
                            sp -= 1;
                            st(b, slot!(sp - 1), v);
                            next!($op);
                        }};
                    }
                    macro_rules! no_float {
                        () => {
                            |_: f64, _: f64| -> Option<Value> { None }
                        };
                    }
                    // Enters the frame of the closure template `t` called at `base` with `argc`
                    // arguments for instruction `$op` (`push_frame` with `PushResult`), or
                    // declines when the template needs the general path or the stack is full.
                    // `$slow`, given the callee's template: what to do with a callee the
                    // loop makes no frame for; true if the call is done.
                    macro_rules! call_closure {
                        ($size:expr, $fuel:expr, $base:expr, $argc:expr, |$fl:ident| $slow:block) => {{
                            let (base, argc): (u32, u32) = ($base, $argc);
                            let Some(f) = ld(b, slot!(base)).as_object() else {
                                break Exit::Decline(op);
                            };
                            if ld32(b, f.offset() + Object::CLASS.offset)
                                != ObjectClass::Function as u32
                            {
                                break Exit::Decline(op);
                            }
                            // The template is in the ELEMENTS field (D2448).
                            let t = ld32(b, f.offset() + Object::ELEMENTS.offset);
                            let TemplateHead {
                                flags,
                                params,
                                locals,
                                stack,
                                code_len,
                                consts_ref,
                            } = template_head(b, t);
                            let done: bool = if flags & slow_call_flags != 0 {
                                let $fl = t;
                                $slow
                            } else {
                                false
                            };
                            if !done {
                                let n = argc.max(params);
                                let new_hp = base + 2 + n;
                                let new_lp = new_hp + HEADER;
                                if new_lp + locals + stack > cap
                                    || !fetchable(b, FuncTemplate::DATA.at(t, 0) + code_len)
                                {
                                    break Exit::Decline(op);
                                }
                                fuel -= $fuel;
                                let ret_pc = ip + $size - code;
                                write_frame(
                                    b,
                                    vals,
                                    (base, argc, n, new_hp, locals),
                                    [
                                        Value::int(bp as i32),
                                        Value::int(hp as i32),
                                        Value::int(ret_pc as i32),
                                        Value::int(argc as i32),
                                        Value::int(PUSH_RESULT),
                                        Value::UNDEFINED,
                                    ],
                                );
                                self.bp = base;
                                self.hp = new_hp;
                                self.lp = new_lp;
                                self.template = t;
                                self.code = FuncTemplate::DATA.at(t, 0);
                                self.code_len = code_len;
                                self.consts = consts_ref;
                                self.native = 0;
                                code = self.code;
                                end = code + code_len;
                                (bp, hp, lp) = (base, new_hp, new_lp);
                                consts = consts_ref;
                                sp = new_lp + locals;
                                ip = code;
                                // Frame entry is a safepoint.
                                if collect {
                                    break Exit::Safepoint;
                                }
                            }
                        }};
                    }
                    // The short forms (D2443): `$i` the implied or one-byte operand.
                    macro_rules! get_loc {
                        ($i:expr, $op:expr) => {{
                            push!(ld(b, slot!(lp + $i)));
                            next!($op);
                        }};
                    }
                    macro_rules! put_loc {
                        ($i:expr, $op:expr) => {{
                            sp -= 1;
                            let v = ld(b, slot!(sp));
                            st(b, slot!(lp + $i), v);
                            next!($op);
                        }};
                    }
                    macro_rules! set_loc {
                        ($i:expr, $op:expr) => {{
                            let v = top!(0);
                            st(b, slot!(lp + $i), v);
                            next!($op);
                        }};
                    }
                    macro_rules! get_arg {
                        ($i:expr, $op:expr) => {{
                            push!(ld(b, slot!(bp + 2 + $i)));
                            next!($op);
                        }};
                    }
                    macro_rules! push_int {
                        ($i:expr, $op:expr) => {{
                            push!(Value::int($i));
                            next!($op);
                        }};
                    }
                    // A short jump (`jump!` with the offset byte).
                    macro_rules! jump8 {
                        ($op:expr, $taken:expr) => {{
                            let info = &OPS[$op as usize];
                            fuel -= info.cost.fuel() as i64;
                            let next = ip + info.size as u32;
                            if !$taken {
                                ip = next;
                            } else {
                                ip = (next as i64 + imm16!() as u8 as i8 as i64) as u32;
                                if ip < next {
                                    if collect {
                                        break Exit::Safepoint;
                                    }
                                    if fuel < floor {
                                        break Exit::Budget;
                                    }
                                }
                            }
                        }};
                    }
                    match op {
                        fast::GET_LOC => {
                            push!(ld(b, slot!(lp + imm16!())));
                            next!(Op::GetLoc);
                        }
                        fast::PUT_LOC => {
                            sp -= 1;
                            let v = ld(b, slot!(sp));
                            st(b, slot!(lp + imm16!()), v);
                            next!(Op::PutLoc);
                        }
                        fast::SET_LOC => {
                            let v = top!(0);
                            st(b, slot!(lp + imm16!()), v);
                            next!(Op::SetLoc);
                        }
                        fast::GET_ARG => {
                            push!(ld(b, slot!(bp + 2 + imm16!())));
                            next!(Op::GetArg);
                        }
                        fast::GET_BUILTIN => {
                            // The realm's prelude function table (D2440).
                            let Some(r) = ld(b, Roots::REALM.offset).as_internal() else {
                                break Exit::Decline(op);
                            };
                            let at = Realm::INTRINSICS.at(r, Intrinsic::PreludeFunctions as u32);
                            let Some(t) = ld(b, at).as_internal() else {
                                break Exit::Decline(op);
                            };
                            if imm16!() >= ld32(b, t + ValueArray::LEN.offset) {
                                break Exit::Decline(op);
                            }
                            push!(ld(b, ValueArray::VALUES.at(t, imm16!())));
                            next!(Op::GetBuiltin);
                        }
                        fast::GET_CAP
                        | fast::GET_CAP_CHK
                        | fast::PUT_CAP
                        | fast::PUT_CAP_CHK
                        | fast::SET_CAP
                        | fast::SET_CAP_CHK
                        | fast::GET_BOX
                        | fast::GET_BOX_CHK
                        | fast::PUT_BOX
                        | fast::PUT_BOX_CHK
                        | fast::SET_BOX
                        | fast::SET_BOX_CHK => {
                            let bx = if matches!(
                                op,
                                fast::GET_BOX
                                    | fast::GET_BOX_CHK
                                    | fast::PUT_BOX
                                    | fast::PUT_BOX_CHK
                                    | fast::SET_BOX
                                    | fast::SET_BOX_CHK
                            ) {
                                ld(b, slot!(lp + imm16!()))
                            } else {
                                // The Box in slot i of the closure (stack[bp]).
                                let Some(f) = ld(b, slot!(bp)).as_object() else {
                                    break Exit::Decline(op);
                                };
                                ld(b, object_slot(b, f.offset(), imm16!()))
                            };
                            let at = bx.as_internal().expect("a Box")
                                + crate::heap::cells::BoxCell::VALUE.offset;
                            let chk = matches!(
                                op,
                                fast::GET_CAP_CHK
                                    | fast::PUT_CAP_CHK
                                    | fast::SET_CAP_CHK
                                    | fast::GET_BOX_CHK
                                    | fast::PUT_BOX_CHK
                                    | fast::SET_BOX_CHK
                            );
                            // The checked forms throw on the TDZ marker: `step` does that.
                            // (The unchecked ones never see it.)
                            let old = ld(b, at);
                            if chk && old.is_uninitialized() {
                                break Exit::Decline(op);
                            }
                            match op {
                                fast::PUT_CAP
                                | fast::PUT_CAP_CHK
                                | fast::PUT_BOX
                                | fast::PUT_BOX_CHK => {
                                    sp -= 1;
                                    let v = ld(b, slot!(sp));
                                    st(b, at, v);
                                }
                                fast::SET_CAP
                                | fast::SET_CAP_CHK
                                | fast::SET_BOX
                                | fast::SET_BOX_CHK => {
                                    let v = top!(0);
                                    st(b, at, v);
                                }
                                _ => push!(old),
                            }
                            next!(op, like Op::GetCap);
                        }
                        fast::PUSH_I8 => {
                            push!(Value::int(imm32!() as u8 as i8 as i32));
                            next!(Op::PushI8);
                        }
                        fast::PUSH_I32 => {
                            push!(Value::int(imm32!() as i32));
                            next!(Op::PushI32);
                        }
                        fast::PUSH_CONST => {
                            assert!(consts != 0, "no constant pool");
                            push!(ld(b, ValueArray::VALUES.at(consts, imm32!())));
                            next!(Op::PushConst);
                        }
                        fast::UNDEFINED => {
                            push!(Value::UNDEFINED);
                            next!(Op::Undefined);
                        }
                        fast::NULL => {
                            push!(Value::NULL);
                            next!(Op::Null);
                        }
                        fast::PUSH_TRUE => {
                            push!(Value::TRUE);
                            next!(Op::PushTrue);
                        }
                        fast::PUSH_FALSE => {
                            push!(Value::FALSE);
                            next!(Op::PushFalse);
                        }
                        fast::SPECIAL_OBJECT => {
                            if imm32!() as u8 != special::THIS {
                                break Exit::Decline(op);
                            }
                            push!(ld(b, slot!(bp + 1)));
                            next!(Op::SpecialObject);
                        }
                        fast::DROP => {
                            sp -= 1;
                            next!(Op::Drop);
                        }
                        fast::DUP => {
                            push!(top!(0));
                            next!(Op::Dup);
                        }
                        fast::DUP2 => {
                            let (a, c) = (top!(1), top!(0));
                            push!(a);
                            push!(c);
                            next!(Op::Dup2);
                        }
                        fast::SWAP => {
                            let (a, c) = (top!(1), top!(0));
                            st(b, slot!(sp - 2), c);
                            st(b, slot!(sp - 1), a);
                            next!(Op::Swap);
                        }
                        fast::NIP => {
                            sp -= 1;
                            let c = ld(b, slot!(sp));
                            st(b, slot!(sp - 1), c);
                            next!(Op::Nip);
                        }
                        fast::INSERT2 => {
                            let (a, c) = (top!(1), top!(0));
                            st(b, slot!(sp - 2), c);
                            st(b, slot!(sp - 1), a);
                            push!(c);
                            next!(Op::Insert2);
                        }
                        // Number operands only: these never need `primitive_operands` (D1280).
                        fast::TO_NUMERIC => {
                            if !top!(0).is_number() {
                                break Exit::Decline(op);
                            }
                            next!(Op::ToNumeric);
                        }
                        // `typeof x`: one of eight strings, each its atom's (D3472).
                        TYPEOF => {
                            let v = top!(0);
                            let Some(s) = heap_call!(self.typeof_hit(v)) else {
                                break Exit::Decline(TYPEOF);
                            };
                            st(b, slot!(sp - 1), s);
                            next!(Op::Typeof);
                        }
                        // A template literal's `${s}` with a string: nothing to do.
                        TO_STRING => {
                            if top!(0).as_string().is_none() {
                                break Exit::Decline(TO_STRING);
                            }
                            next!(Op::ToString);
                        }
                        fast::INC | fast::DEC => {
                            let d = if op == fast::INC { 1 } else { -1 };
                            let Some(i) = top!(0).as_int().and_then(|i| i.checked_add(d)) else {
                                break Exit::Decline(op);
                            };
                            st(b, slot!(sp - 1), Value::int(i));
                            next!(Op::Inc);
                        }
                        fast::INC_LOC | fast::DEC_LOC => {
                            let d = if op == fast::INC_LOC { 1 } else { -1 };
                            let at = slot!(lp + imm16!());
                            let Some(i) = ld(b, at).as_int().and_then(|i| i.checked_add(d)) else {
                                break Exit::Decline(op);
                            };
                            st(b, at, Value::int(i));
                            next!(Op::IncLoc);
                        }
                        fast::ADD => binary!(
                            Op::Add,
                            |x: i32, y: i32| x.checked_add(y).map(Value::int),
                            |x: f64, y: f64| Some(conv::number(x + y))
                        ),
                        fast::SUB => binary!(
                            Op::Sub,
                            |x: i32, y: i32| x.checked_sub(y).map(Value::int),
                            |x: f64, y: f64| Some(conv::number(x - y))
                        ),
                        // An int32 product of 0 may be -0 (`-1 * 0`): the f64 path gets it right.
                        fast::MUL => binary!(
                            Op::Mul,
                            |x: i32, y: i32| x.checked_mul(y).filter(|&r| r != 0).map(Value::int),
                            |x: f64, y: f64| Some(conv::number(x * y))
                        ),
                        fast::DIV => {
                            binary!(Op::Div, |_: i32, _: i32| None, |x: f64, y: f64| Some(
                                conv::number(x / y)
                            ))
                        }
                        fast::MOD => binary!(
                            Op::Mod,
                            |x: i32, y: i32| (x >= 0 && y != 0).then(|| Value::int(x % y)),
                            no_float!()
                        ),
                        fast::BAND => {
                            binary!(
                                Op::BAnd,
                                |x: i32, y: i32| Some(Value::int(x & y)),
                                no_float!()
                            )
                        }
                        fast::BOR => {
                            binary!(
                                Op::BOr,
                                |x: i32, y: i32| Some(Value::int(x | y)),
                                no_float!()
                            )
                        }
                        fast::BXOR => {
                            binary!(
                                Op::BXor,
                                |x: i32, y: i32| Some(Value::int(x ^ y)),
                                no_float!()
                            )
                        }
                        fast::SHL => binary!(
                            Op::Shl,
                            |x: i32, y: i32| Some(Value::int(x.wrapping_shl(y as u32 & 31))),
                            no_float!()
                        ),
                        fast::SAR => binary!(
                            Op::Sar,
                            |x: i32, y: i32| Some(Value::int(x >> (y as u32 & 31))),
                            no_float!()
                        ),
                        // NaN compares false.
                        fast::LT => binary!(
                            Op::Lt,
                            |x: i32, y: i32| Some(Value::bool(x < y)),
                            |x: f64, y: f64| Some(Value::bool(x < y))
                        ),
                        fast::LE => binary!(
                            Op::Le,
                            |x: i32, y: i32| Some(Value::bool(x <= y)),
                            |x: f64, y: f64| Some(Value::bool(x <= y))
                        ),
                        fast::GT => binary!(
                            Op::Gt,
                            |x: i32, y: i32| Some(Value::bool(x > y)),
                            |x: f64, y: f64| Some(Value::bool(x > y))
                        ),
                        fast::GE => binary!(
                            Op::Ge,
                            |x: i32, y: i32| Some(Value::bool(x >= y)),
                            |x: f64, y: f64| Some(Value::bool(x >= y))
                        ),
                        fast::STRICT_EQ | fast::STRICT_NE => {
                            let (a, c) = (top!(1), top!(0));
                            let eq = match (a.as_number(), c.as_number()) {
                                (Some(x), Some(y)) => x == y,
                                _ if a == c => true,
                                // Two flat one-byte strings (`kind === "text"`): read
                                // from the slice.
                                _ if let (Some(x), Some(y)) = (a.as_string(), c.as_string())
                                    && let Some(eq) = hot_str_eq(b, x.0, y.0) =>
                                {
                                    #[cfg(debug_assertions)]
                                    {
                                        // (Charging nothing: release builds do
                                        // not compare twice.)
                                        let want = heap_call_op!({
                                            let w = self.heap.work_mark();
                                            let r = conv::strict_equals(self.heap, a, c);
                                            self.heap.work_reset(w);
                                            r
                                        });
                                        assert!(want == eq, "the slice's string compare differs");
                                    }
                                    eq
                                }
                                _ if (a.as_string().is_some() && c.as_string().is_some())
                                    || (a.as_bigint().is_some() && c.as_bigint().is_some()) =>
                                {
                                    heap_call_op!(conv::strict_equals(self.heap, a, c))
                                }
                                _ => false,
                            };
                            sp -= 1;
                            st(b, slot!(sp - 1), Value::bool(eq == (op == fast::STRICT_EQ)));
                            // (`strict_ne` has the same size and cost.)
                            next!(Op::StrictEq);
                        }
                        fast::NOT => {
                            let a = top!(0);
                            let truthy = if let Some(x) = a.as_bool() {
                                x
                            } else if let Some(n) = a.as_number() {
                                !(n == 0.0 || n.is_nan())
                            } else if a.is_undefined() || a.is_null() {
                                false
                            } else if a.as_object().is_some() {
                                true
                            } else {
                                heap_call!(conv::to_boolean(self.heap, a))
                            };
                            st(b, slot!(sp - 1), Value::bool(!truthy));
                            next!(Op::Not);
                        }
                        fast::IS_UNDEFINED => {
                            let v = Value::bool(top!(0).is_undefined());
                            st(b, slot!(sp - 1), v);
                            next!(Op::IsUndefined);
                        }
                        fast::IS_NULLISH => {
                            let v = Value::bool(top!(0).is_nullish());
                            st(b, slot!(sp - 1), v);
                            next!(Op::IsNullish);
                        }
                        fast::JUMP => jump!(Op::Jump, true),
                        fast::JUMP_IF_FALSE | fast::JUMP_IF_TRUE => {
                            let Some(c) = top!(0).as_bool() else {
                                break Exit::Decline(op);
                            };
                            sp -= 1;
                            // (One copy of the jump for both; the short forms, which
                            // loops use, keep theirs.)
                            jump!(Op::JumpIfTrue, c == (op == fast::JUMP_IF_TRUE))
                        }
                        fast::GET_FIELD | fast::GET_METHOD => {
                            let key = Atom::from_id(imm32!());
                            let ic = (w >> 40) as u16 as u32;
                            let o = top!(0);
                            // The frequent hits are read from the slice (D3393); the
                            // others, and the misses, out of line.
                            let hit = if key == atoms::LENGTH && op == fast::GET_FIELD {
                                hot_length(b, o)
                            } else {
                                super::ic::hot_get(b, self.template, ic, o, key)
                            };
                            let r = match hit {
                                Some(r) => {
                                    #[cfg(debug_assertions)]
                                    {
                                        let want = heap_call_op!({
                                            let w = self.heap.work_mark();
                                            let r = self.field_slow(ip - code, o, false);
                                            self.heap.work_reset(w);
                                            r
                                        });
                                        assert!(
                                            want == Some(Hit::Value(r)),
                                            "the slice's hit differs"
                                        );
                                    }
                                    r
                                }
                                None => {
                                    match heap_call_op!(self.field_slow(
                                        ip - code,
                                        o,
                                        sp + 2 <= cap
                                    )) {
                                        Some(Hit::Value(r)) => r,
                                        // A getter with a `@fast` entry, which ran: the
                                        // fuel of such a call (D3530).
                                        Some(Hit::Fast(r)) => {
                                            fuel -= super::fastpath::FAST_FUEL;
                                            // (`call_accessor` pushes the receiver above
                                            // the getter: the same bytes in that slot.)
                                            st(b, slot!(sp), o);
                                            r
                                        }
                                        // An accessor (D3470): `decline_fast` calls the
                                        // getter, which `field_slow` left in `hot_getter`.
                                        // (Written with the opcode's test, which
                                        // `field_slow` has made already: with the bare
                                        // constant V8 moved the arithmetic arms, D3470.)
                                        Some(Hit::Getter(_)) => {
                                            if op == fast::GET_FIELD {
                                                break Exit::Decline(GETTER);
                                            }
                                            break Exit::Decline(fast::GET_METHOD);
                                        }
                                        None => break Exit::Decline(op),
                                    }
                                }
                            };
                            st(b, slot!(sp - 1), r);
                            if op == fast::GET_METHOD {
                                push!(o);
                            }
                            next!(op, like Op::GetField);
                        }
                        fast::PUT_FIELD | fast::DEFINE_FIELD => {
                            let key = Atom::from_id(imm32!());
                            let ic = (w >> 40) as u16 as u32;
                            let (o, v) = (top!(1), top!(0));
                            if !heap_call_op!(self.field_store(op, key, ic, o, v)) {
                                break Exit::Decline(op);
                            }
                            // `obj v -> obj` for `define_field`.
                            sp -= if op == fast::PUT_FIELD { 2 } else { 1 };
                            next!(op, like Op::PutField);
                        }
                        fast::GET_ELEM | fast::GET_ELEM2 => {
                            let (o, key) = (top!(1), top!(0));
                            // A dense element is read from the slice (D3393); the rest
                            // out of line.
                            let r = match hot_elem(b, o, key) {
                                Some(r) => {
                                    #[cfg(debug_assertions)]
                                    {
                                        let want = heap_call_op!({
                                            let w = self.heap.work_mark();
                                            let r = self.elem_slow(false, o, key, r);
                                            self.heap.work_reset(w);
                                            r
                                        });
                                        assert!(want == Some(r), "the slice's element differs");
                                    }
                                    r
                                }
                                None => {
                                    let hit = heap_call_op!(self.elem_slow(
                                        false,
                                        o,
                                        key,
                                        Value::UNDEFINED
                                    ));
                                    let Some(r) = hit else {
                                        break Exit::Decline(op);
                                    };
                                    r
                                }
                            };
                            if op == fast::GET_ELEM2 {
                                push!(r);
                                next!(Op::GetElem2);
                            } else {
                                sp -= 1;
                                st(b, slot!(sp - 1), r);
                                next!(Op::GetElem);
                            }
                        }
                        fast::PUT_ELEM => {
                            let (o, key, v) = (top!(2), top!(1), top!(0));
                            let index = key.as_int().and_then(|i| u32::try_from(i).ok());
                            let done = match (o.as_object(), index) {
                                // (In line, as it was: out of line a store into an
                                // array cost 28 machine instructions more.)
                                (Some(obj), Some(i)) => heap_call!(matches!(
                                    self.heap.set_element_fast(obj, i, v),
                                    Ok(true)
                                )),
                                _ => heap_call!(self.elem_slow(true, o, key, v)).is_some(),
                            };
                            if !done {
                                break Exit::Decline(fast::PUT_ELEM);
                            }
                            sp -= 3;
                            next!(Op::PutElem);
                        }
                        fast::ITER_NEXT => {
                            let (it, f) = (top!(1), top!(0));
                            let Some(v) = heap_call!(self.iter_next_hit(it, f)) else {
                                break Exit::Decline(fast::ITER_NEXT);
                            };
                            sp -= 1;
                            st(b, slot!(sp - 1), v);
                            // (The offset read again: `w` kept across the call is
                            // spilled for every instruction, D2441.)
                            jump!(Op::IterNext, true, ld32(b, ip + 1) as i32)
                        }
                        fast::ITER_NEXT_LOC => {
                            let at = lp + imm16!();
                            let (it, f) = (ld(b, slot!(at)), ld(b, slot!(at + 1)));
                            let Some(v) = heap_call!(self.iter_next_hit(it, f)) else {
                                break Exit::Decline(fast::ITER_NEXT_LOC);
                            };
                            push!(v);
                            jump!(Op::IterNextLoc, true, ld32(b, ip + 3) as i32)
                        }
                        fast::CALL_INTRINSIC | fast::CALL_INTRINSIC8 => {
                            // (One arm for both: two copies of `intrinsic_hit` are a
                            // longer V8 tier-up, D2443.)
                            let (id, argc, size) = if op == fast::CALL_INTRINSIC {
                                let argc = (w >> 24) as u16 as u32;
                                (imm16!() as u16, argc, size_of(Op::CallIntrinsic))
                            } else {
                                let id = imm16!() & 0xFF;
                                (id as u16, arity(id), size_of(Op::CallIntrinsic8))
                            };
                            const FUEL: i64 = fuel_of(Op::CallIntrinsic);
                            let base = sp - argc;
                            if id == CALL && argc >= 2 {
                                // `f this a1 .. an`, as in `call_intrinsic`.
                                call_closure!(size, FUEL, base, argc - 2, |_f| {
                                    break Exit::Decline(op);
                                });
                            } else {
                                let mut a = [Value::UNDEFINED; 3];
                                for (k, x) in a.iter_mut().enumerate().take(argc.min(3) as usize) {
                                    *x = ld(b, slot!(base + k as u32));
                                }
                                let Some(v) = heap_call_op!(self.intrinsic_hit(id, argc, a)) else {
                                    // A host refusal (`pure_leaf_hit`) stops the run here.
                                    if self.host_oom {
                                        fuel = 0;
                                    }
                                    break Exit::Decline(op);
                                };
                                sp = base;
                                push!(v);
                                fuel -= FUEL;
                                ip += size;
                            }
                        }
                        // The global caches (D1620); the misses run `step`.
                        // (One arm each, naming its opcode: as one arm that read the
                        // opcode again after the call, V8 moved a value on the path of
                        // the simple instructions, D3392.)
                        GET_GLOBAL => {
                            let (key, ic) = (Atom::from_id(imm32!()), (w >> 40) as u16 as u32);
                            let Some(v) = heap_call!(self.ic_global_get(key, ic)) else {
                                break Exit::Decline(GET_GLOBAL);
                            };
                            push!(v);
                            next!(Op::GetGlobal);
                        }
                        GET_GLOBAL_TYPEOF => {
                            let (key, ic) = (Atom::from_id(imm32!()), (w >> 40) as u16 as u32);
                            let Some(v) = heap_call!(self.ic_global_get(key, ic)) else {
                                break Exit::Decline(GET_GLOBAL_TYPEOF);
                            };
                            push!(v);
                            next!(Op::GetGlobalTypeof);
                        }
                        PUT_GLOBAL => {
                            let (key, ic) = (Atom::from_id(imm32!()), (w >> 40) as u16 as u32);
                            let v = top!(0);
                            if !heap_call!(self.ic_global_put(key, ic, v)) {
                                break Exit::Decline(PUT_GLOBAL);
                            }
                            sp -= 1;
                            next!(Op::PutGlobal);
                        }
                        fast::CALL | fast::CALL0 | fast::CALL1 | fast::CALL2 | fast::CALL3 => {
                            let (argc, size) = if op == fast::CALL {
                                (imm16!(), size_of(Op::Call))
                            } else {
                                ((op - fast::CALL0) as u32, size_of(Op::Call0))
                            };
                            let base = sp - 2 - argc;
                            call_closure!(size, fuel_of(Op::Call), base, argc, |t| {
                                // A `@fast` builtin: its call without leaving the
                                // loop (D3551). One that declines made its attempt:
                                // the call is `step`'s, not `decline_fast`'s. (Nothing
                                // but ip, sp and fuel is kept across the call, D3391:
                                // it answers with what to take from sp and the size.)
                                // Near the stack's end a leaf's intrinsic might
                                // move the stack (`n_leaf`): out of the loop then.
                                // (The flags read again: with the test on the
                                // frame's copy the closure's path loads them as
                                // two bytes, +3 instructions.)
                                if ld32(b, t + FuncTemplate::FLAGS.offset) & super::func_flags::FAST
                                    == 0
                                    || sp + super::fastpath::LEAF_ROOM > cap
                                {
                                    break Exit::Decline(op);
                                }
                                let r = heap_call!(self.loop_leaf_call(base, argc, ip - code));
                                if r == 0 {
                                    break Exit::General;
                                }
                                debug_assert!(
                                    vals == ValueArray::VALUES.at(self.stack, 0),
                                    "a leaf moved the stack"
                                );
                                sp -= r as u32;
                                fuel -= const { fuel_of(Op::Call) + super::fastpath::FAST_FUEL };
                                ip += (r >> 32) as u32;
                                true
                            });
                        }
                        fast::RETURN | fast::RETURN_UNDEF => {
                            // `fast_return`: into a PushResult frame of a bytecode caller.
                            let flags = ld(b, slot!(hp + frame::FLAGS)).as_int().unwrap_or(0);
                            if flags != PUSH_RESULT {
                                if flags != EACH {
                                    break Exit::Decline(op);
                                }
                                let v = if op == fast::RETURN {
                                    top!(0)
                                } else {
                                    Value::UNDEFINED
                                };
                                fuel -= fuel_of(Op::Return);
                                break Exit::Each(v);
                            }
                            let int = |v: Value| v.as_int().unwrap_or(0) as u32;
                            let h: [u8; 48] = window(b, slot!(hp));
                            let hv = |k: u32| {
                                let k = 8 * k as usize;
                                Value::from_raw(u64::from_le_bytes(h[k..k + 8].try_into().unwrap()))
                            };
                            let (prev_bp, prev_hp, ret_pc) = (
                                int(hv(frame::PREV_BP)),
                                int(hv(frame::PREV_HP)),
                                int(hv(frame::RET_PC)),
                            );
                            // A native caller (D1631) is entered as `load_frame` enters it
                            // (`code_len` 0, so the next round goes to `native_step`).
                            let native = int(ld(b, slot!(prev_hp + frame::FLAGS))) as i32
                                & frame::FLAG_NATIVE
                                != 0;
                            let f = ld(b, slot!(prev_bp)).as_object().expect("frame callee");
                            let t = ld32(b, f.offset() + Object::ELEMENTS.offset);
                            let code_len = ld32(b, t + FuncTemplate::CODE_LEN.offset);
                            let consts_ref = ld32(b, t + FuncTemplate::CONSTS.offset);
                            let v = if op == fast::RETURN {
                                top!(0)
                            } else {
                                Value::UNDEFINED
                            };
                            fuel -= fuel_of(Op::Return);
                            // pop_frame, push the result.
                            st(b, slot!(bp), v);
                            sp = bp + 1;
                            self.bp = prev_bp;
                            self.hp = prev_hp;
                            self.lp = prev_hp + HEADER;
                            self.template = t;
                            self.code = FuncTemplate::DATA.at(t, 0);
                            self.code_len = code_len;
                            self.consts = consts_ref;
                            self.native = 0;
                            if native {
                                // FLAGS, NATIVE and FAST in one word (D2448).
                                const _: () = assert!(
                                    FuncTemplate::NATIVE.offset == FuncTemplate::FLAGS.offset + 2
                                );
                                self.native =
                                    (ld32(b, t + FuncTemplate::FLAGS.offset) >> 16 & 0xFF) + 1;
                                self.code_len = 0;
                            }
                            code = self.code;
                            end = code + self.code_len;
                            (bp, hp, lp) = (prev_bp, prev_hp, prev_hp + HEADER);
                            consts = consts_ref;
                            ip = code + ret_pc;
                            // A return is a safepoint.
                            if collect {
                                break Exit::Safepoint;
                            }
                            // A native caller (`code_len` 0) is the general path's.
                            if native || !fetchable(b, end) {
                                break Exit::General;
                            }
                        }
                        // The short forms (D2443), after the others and one arm
                        // each: in the wasm build, merged arms and ranges (a second
                        // dispatch) cost the other instructions measurably.
                        fast::GET_LOC0 | fast::GET_LOC1 | fast::GET_LOC2 | fast::GET_LOC3 => {
                            get_loc!((op - fast::GET_LOC0) as u32, Op::GetLoc0)
                        }
                        fast::GET_LOC8 => get_loc!(imm16!() & 0xFF, Op::GetLoc8),
                        fast::PUT_LOC0 | fast::PUT_LOC1 | fast::PUT_LOC2 | fast::PUT_LOC3 => {
                            put_loc!((op - fast::PUT_LOC0) as u32, Op::PutLoc0)
                        }
                        fast::PUT_LOC8 => put_loc!(imm16!() & 0xFF, Op::PutLoc8),
                        fast::SET_LOC0 | fast::SET_LOC1 | fast::SET_LOC2 | fast::SET_LOC3 => {
                            set_loc!((op - fast::SET_LOC0) as u32, Op::SetLoc0)
                        }
                        fast::SET_LOC8 => set_loc!(imm16!() & 0xFF, Op::SetLoc8),
                        fast::GET_ARG0 | fast::GET_ARG1 | fast::GET_ARG2 | fast::GET_ARG3 => {
                            get_arg!((op - fast::GET_ARG0) as u32, Op::GetArg0)
                        }
                        fast::GET_ARG8 => get_arg!(imm16!() & 0xFF, Op::GetArg8),
                        fast::PUSH0 | fast::PUSH1 | fast::PUSH2 | fast::PUSH3 => {
                            push_int!((op - fast::PUSH0) as i32, Op::Push0)
                        }
                        fast::PUSH_CONST8 => {
                            assert!(consts != 0, "no constant pool");
                            push!(ld(b, ValueArray::VALUES.at(consts, imm16!() & 0xFF)));
                            next!(Op::PushConst8);
                        }
                        fast::JUMP8 => jump8!(Op::Jump8, true),
                        fast::JUMP_IF_FALSE8 | fast::JUMP_IF_TRUE8 => {
                            let Some(c) = top!(0).as_bool() else {
                                break Exit::Decline(op);
                            };
                            sp -= 1;
                            // (Two copies of the jump: one for both costs a loop an
                            // instruction or two per iteration.)
                            if op == fast::JUMP_IF_TRUE8 {
                                jump8!(Op::JumpIfTrue8, c)
                            } else {
                                jump8!(Op::JumpIfFalse8, !c)
                            }
                        }
                        fast::GET_BUILTIN_CALLEE8 => {
                            let i = imm16!() & 0xFF;
                            let Some(r) = ld(b, Roots::REALM.offset).as_internal() else {
                                break Exit::Decline(op);
                            };
                            let at = Realm::INTRINSICS.at(r, Intrinsic::PreludeFunctions as u32);
                            let Some(t) = ld(b, at).as_internal() else {
                                break Exit::Decline(op);
                            };
                            if i >= ld32(b, t + ValueArray::LEN.offset) {
                                break Exit::Decline(op);
                            }
                            push!(ld(b, ValueArray::VALUES.at(t, i)));
                            push!(Value::UNDEFINED);
                            next!(Op::GetBuiltinCallee8);
                        }
                        _ => break Exit::Decline(op),
                    }
                }
            };
            self.pc = ip - code;
            self.sp = sp;
            self.fuel = fuel;
            match exit {
                Exit::Each(v) => {
                    if let Some(st) = self.each_return(v) {
                        return Some(st);
                    }
                }
                Exit::Safepoint => self.safepoint(),
                Exit::Budget => return None,
                Exit::Decline(_) | Exit::General => {
                    let op = match exit {
                        Exit::Decline(op) => op,
                        _ => 0,
                    };
                    // The instruction started with fuel left: an attempt that declined
                    // may have charged its work (0289), but the instruction completes
                    // before the run stops (else a slice smaller than the attempt's work
                    // would make it again at every resume). A host refusal (which set
                    // the fuel to 0) stops here.
                    if self.host_oom && self.fuel <= 0 {
                        return None;
                    }
                    // A call of a `@fast` builtin (D1850) or `%EachDense` (D2307), else the
                    // general path.
                    if (op == fast::CALL_INTRINSIC || op == fast::CALL_INTRINSIC8)
                        && let Some(r) = self.each_dense_decline()
                    {
                        if let Some(st) = r {
                            return Some(st);
                        }
                    } else if !self.decline_fast(op) {
                        if let Some(st) = self.step_once() {
                            return Some(st);
                        }
                        if self.fuel < floor {
                            return None;
                        }
                    }
                }
            }
            // The work of what ran outside the loop (0289).
            self.fuel -= self.heap.take_work();
            if self.fuel <= 0 {
                return None;
            }
            // `step` may have collected, grown the stack or changed the frame.
            load_frame!();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fetchable_needs_a_whole_word_for_the_last_instruction() {
        let b = [0u8; 16];
        // The last instruction may be the single byte at `end - 1`: its word ends at
        // `end + 7`.
        assert!(super::fetchable(&b, 9));
        assert!(!super::fetchable(&b, 10));
        assert!(!super::fetchable(&b, u32::MAX));
        assert_eq!(super::word(&b, 8), 0);
    }
}
