//! djs-wasm: the `wasm32-unknown-unknown` export layer over djs-core's [`Vm`]
//! (ARCH "Execution API"). Only integer and pointer arguments; no wasm-bindgen. The ABI is
//! documented in docs/host.md, and js/src/sandbox.ts is its client.
//!
//! Data crosses the boundary through two buffers in linear memory: the *staging* buffer
//! (host → wasm: sources, names, wire data, argument handles), sized with `djs_staging`,
//! and the *out* buffer (wasm → host: messages, display text, value bits, host call
//! records), read at `djs_out_ptr` / `djs_out_len` after the call that filled it. Both
//! pointers are valid until the next call into the module.
//!
//! VMs are numbered from 1; values that must survive a call are heap handles, numbered
//! from 1 (`handle index + 1`; 0 means `undefined` where a value is expected). Using a
//! freed VM or handle number is a trap.
//!
//! Panics trap (wasm32-unknown-unknown aborts on panic). A hook records the message
//! first, so the host can read it with `djs_panic_ptr` / `djs_panic_len` afterwards. After
//! a trap the instance must not be used again.

use std::cell::RefCell;

use djs_core::compiler::{CompileOptions, Goal};
use djs_core::heap::Handle;
use djs_core::interp::{ErrorKind, Intrinsic, intrinsic};
use djs_core::json;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};
use djs_core::wire::{self, DecodeOptions};

/// The ABI version, bumped on any incompatible change to the exports.
pub const ABI_VERSION: u32 = 1;

/// `vm_run` results.
pub mod status {
    pub const DONE: u32 = 0;
    pub const THREW: u32 = 1;
    pub const OUT_OF_FUEL: u32 = 2;
    pub const HOST_CALL: u32 = 3;
    pub const IDLE: u32 = 4;
    /// The host could not provide memory below the cap (D1690): not a JS error.
    pub const HOST_OUT_OF_MEMORY: u32 = 5;
}

/// One VM and what the last `vm_run` reported.
struct Slot {
    vm: Vm,
    /// Done/Threw: a handle (index + 1) to the value, until the host takes it.
    result: u32,
    /// HostCall: the call id the VM is parked on.
    call_id: u32,
}

#[derive(Default)]
struct State {
    vms: Vec<Option<Slot>>,
    staging: Vec<u8>,
    out: Vec<u8>,
}

thread_local! {
    // `const`: no lazy-initialization code in each of the ~40 `with` instantiations
    // (0043: 10 KB of wasm).
    static STATE: RefCell<State> = const {
        RefCell::new(State {
            vms: Vec::new(),
            staging: Vec::new(),
            out: Vec::new(),
        })
    };
    /// The message of the last panic. Separate from `STATE`, which may be left borrowed by
    /// the call that trapped.
    static PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

fn init() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // Natively (tests) the default hook still prints; on wasm32 there is no stderr, and
        // leaving it out keeps its code out of the module.
        #[cfg(not(target_arch = "wasm32"))]
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let msg = info.to_string();
            PANIC.with(|p| {
                if let Ok(mut p) = p.try_borrow_mut() {
                    *p = msg;
                }
            });
            #[cfg(not(target_arch = "wasm32"))]
            previous(info);
        }));
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    init();
    STATE.with(|s| f(&mut s.borrow_mut()))
}

impl State {
    fn slot(&mut self, vm: u32) -> &mut Slot {
        self.vms
            .get_mut((vm as usize).wrapping_sub(1))
            .and_then(Option::as_mut)
            .unwrap_or_else(|| panic!("djs-wasm: no VM {vm}"))
    }

    /// The VM and the staging buffer, borrowed separately.
    fn vm_and_staging(&mut self, vm: u32) -> (&mut Slot, &[u8]) {
        let slot = self
            .vms
            .get_mut((vm as usize).wrapping_sub(1))
            .and_then(Option::as_mut)
            .unwrap_or_else(|| panic!("djs-wasm: no VM {vm}"));
        (slot, &self.staging)
    }

    fn set_out(&mut self, bytes: &[u8]) {
        self.out.clear();
        self.out.extend_from_slice(bytes);
    }

    /// Writes `[call_id, fn_id, this_lo, this_hi, (arg_lo, arg_hi) * argc]` for host call
    /// `id` to the out buffer; returns `argc` (0 with an empty buffer if there is none).
    fn describe_call(&mut self, vm: u32, id: u32) -> u32 {
        let Some(info) = self.slot(vm).vm.host_call(id) else {
            self.out.clear();
            return 0;
        };
        let mut words = vec![id, info.fn_id];
        for v in std::iter::once(info.this).chain(info.args.iter().copied()) {
            let b = v.to_bits();
            words.push(b as u32);
            words.push((b >> 32) as u32);
        }
        self.set_out_words(&words);
        info.args.len() as u32
    }

    fn set_out_words(&mut self, words: &[u32]) {
        self.out.clear();
        for w in words {
            self.out.extend_from_slice(&w.to_le_bytes());
        }
    }

    fn add_vm(&mut self, vm: Vm) -> u32 {
        let slot = Slot {
            vm,
            result: 0,
            call_id: 0,
        };
        match self.vms.iter().position(Option::is_none) {
            Some(i) => {
                self.vms[i] = Some(slot);
                i as u32 + 1
            }
            None => {
                self.vms.push(Some(slot));
                self.vms.len() as u32
            }
        }
    }
}

fn staged(staging: &[u8], start: usize, len: usize) -> &[u8] {
    staging
        .get(start..start + len)
        .unwrap_or_else(|| panic!("djs-wasm: staging range {start}+{len} out of bounds"))
}

fn staged_str(staging: &[u8], start: usize, len: usize) -> &str {
    std::str::from_utf8(staged(staging, start, len))
        .unwrap_or_else(|_| panic!("djs-wasm: staged text is not UTF-8"))
}

impl Slot {
    /// A new handle for `v`, as an ABI number. 0 if the heap is full.
    fn handle(&mut self, v: Value) -> u32 {
        match self.vm.heap_mut().new_handle(v) {
            Ok(h) => h.index() + 1,
            Err(_) => 0,
        }
    }

    /// The value of an ABI handle number (0 is `undefined`).
    fn value(&self, h: u32) -> Value {
        if h == 0 {
            Value::UNDEFINED
        } else {
            self.vm.heap().get_handle(Handle::from_index(h - 1))
        }
    }
}

// ---- Buffers -------------------------------------------------------------------------

/// The ABI version ([`ABI_VERSION`]).
#[unsafe(no_mangle)]
pub extern "C" fn djs_abi_version() -> u32 {
    ABI_VERSION
}

/// The build hash of VM images (`djs_core::vm::BUILD_HASH`): it changes with the engine's
/// sources, so a host can tell builds apart (pi's deterministic fingerprint, 0049).
#[unsafe(no_mangle)]
pub extern "C" fn djs_build_hash() -> u32 {
    djs_core::vm::BUILD_HASH
}

/// Makes the staging buffer at least `len` bytes and returns its address. Valid until the
/// next call.
#[unsafe(no_mangle)]
pub extern "C" fn djs_staging(len: usize) -> usize {
    with(|s| {
        if s.staging.len() < len {
            s.staging.resize(len, 0);
        }
        s.staging.as_mut_ptr() as usize
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn djs_out_ptr() -> usize {
    with(|s| s.out.as_ptr() as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn djs_out_len() -> usize {
    with(|s| s.out.len())
}

/// The message of the last panic (after a trap), UTF-8.
#[unsafe(no_mangle)]
pub extern "C" fn djs_panic_ptr() -> usize {
    PANIC.with(|p| p.borrow().as_ptr() as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn djs_panic_len() -> usize {
    PANIC.with(|p| p.borrow().len())
}

/// Panics on purpose, so hosts can test their trap handling.
#[unsafe(no_mangle)]
pub extern "C" fn djs_debug_panic() {
    init();
    panic!("djs_debug_panic called");
}

// ---- VM lifecycle --------------------------------------------------------------------

fn options(memory_limit: u32) -> VmOptions {
    VmOptions {
        memory_limit: if memory_limit == 0 {
            djs_core::heap::DEFAULT_LIMIT
        } else {
            memory_limit as usize
        },
        ..VmOptions::default()
    }
}

/// A new VM with a heap cap of `memory_limit` bytes (0: the default): a copy of the base
/// snapshot (djs-prelude, D1321). Returns its number, or 0 if the cap is too small for
/// the realm.
#[unsafe(no_mangle)]
pub extern "C" fn vm_new(memory_limit: u32) -> u32 {
    add_new_vm(djs_prelude::new_vm(options(memory_limit)))
}

/// Like `vm_new`, but builds the realm and compiles the prelude instead of copying the
/// base (about 150x slower). For checking the base against the realm built in wasm; only
/// with the `compiled-prelude` feature.
#[cfg(feature = "compiled-prelude")]
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_compiled(memory_limit: u32) -> u32 {
    // Compacted like djs-prelude's build compacts the base (D1431), so that after a
    // collection the two images can be compared byte for byte.
    let vm = Vm::new(options(memory_limit)).and_then(|mut vm| {
        vm.heap_mut().compact_for_image()?;
        Ok(vm)
    });
    add_new_vm(vm)
}

fn add_new_vm(vm: Result<Vm, djs_core::heap::HeapError>) -> u32 {
    with(|s| match vm {
        Ok(vm) => s.add_vm(vm),
        Err(e) => {
            s.set_out(format!("cannot create a VM: {e:?}").as_bytes());
            0
        }
    })
}

/// A VM restored from an image (`vm_arena_*` bytes of the same build) in staging[0..len].
/// Returns its number, or 0 with the reason in the out buffer.
#[unsafe(no_mangle)]
pub extern "C" fn vm_from_bytes(len: usize, memory_limit: u32) -> u32 {
    with(|s| {
        staged(&s.staging, 0, len);
        // The image becomes the arena: hand over the staging buffer instead of copying.
        let mut bytes = std::mem::take(&mut s.staging);
        bytes.truncate(len);
        match Vm::from_bytes(bytes, options(memory_limit)) {
            Ok(vm) => s.add_vm(vm),
            Err(e) => {
                s.set_out(format!("cannot restore the VM image: {e}").as_bytes());
                0
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vm_free(vm: u32) {
    with(|s| {
        s.slot(vm);
        s.vms[vm as usize - 1] = None;
        while s.vms.last().is_some_and(Option::is_none) {
            s.vms.pop();
        }
    })
}

/// The address of the VM's arena (its offset 0). Changes when the arena grows: re-read it
/// after every call that may run or allocate.
#[unsafe(no_mangle)]
pub extern "C" fn vm_arena_ptr(vm: u32) -> usize {
    with(|s| s.slot(vm).vm.bytes().as_ptr() as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn vm_arena_len(vm: u32) -> usize {
    with(|s| s.slot(vm).vm.bytes().len())
}

/// Collects garbage. Returns 0, or -1 if the heap could not reserve the to-space.
#[unsafe(no_mangle)]
pub extern "C" fn vm_collect(vm: u32) -> i32 {
    with(|s| {
        if s.slot(vm).vm.collect().is_ok() {
            0
        } else {
            -1
        }
    })
}

// ---- Code ----------------------------------------------------------------------------

/// Compiles staging[params_len..params_len + src_len] (UTF-8 source). `goal` 0 is a
/// Script; 1 is a codemode async function body whose parameter names are
/// staging[0..params_len], separated by `\n`. `goal | 2` parses TypeScript (0048). Returns a handle to the closure, or 0 with
/// the error (`SyntaxError: ... at line:col`) in the out buffer. Stack traces name the
/// code `sandbox.js`; `vm_compile_file` names it.
#[unsafe(no_mangle)]
pub extern "C" fn vm_compile(vm: u32, goal: u32, params_len: usize, src_len: usize) -> u32 {
    compile(vm, goal, 0, params_len, src_len)
}

/// `vm_compile` with a file name for stack traces (0049: pi's codemode reports
/// `codemode.js:2`): staging holds the name (`name_len` bytes, UTF-8), then the parameter
/// names, then the source.
#[unsafe(no_mangle)]
pub extern "C" fn vm_compile_file(
    vm: u32,
    goal: u32,
    name_len: usize,
    params_len: usize,
    src_len: usize,
) -> u32 {
    compile(vm, goal, name_len, params_len, src_len)
}

fn compile(vm: u32, goal: u32, name_len: usize, params_len: usize, src_len: usize) -> u32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let name = if name_len == 0 {
            String::from("sandbox.js")
        } else {
            staged_str(staging, 0, name_len).to_owned()
        };
        let params_text = staged_str(staging, name_len, params_len).to_owned();
        let src = staged(staging, name_len + params_len, src_len).to_vec();
        let params: Vec<&str> = params_text.split('\n').filter(|p| !p.is_empty()).collect();
        let opts = CompileOptions {
            goal: if goal & 1 == 0 {
                Goal::Script
            } else {
                Goal::AsyncBody
            },
            filename: &name,
            params: &params,
            privileged: false,
            typescript: goal & 2 != 0,
        };
        let result = slot.vm.compile(&src, &opts).map_err(|e| e.to_string());
        match result {
            Ok(f) => {
                let h = slot.handle(f);
                if h == 0 {
                    s.set_out(b"out of memory");
                }
                h
            }
            Err(msg) => {
                s.set_out(msg.as_bytes());
                0
            }
        }
    })
}

// ---- Values in -----------------------------------------------------------------------

/// A handle to the global object.
#[unsafe(no_mangle)]
pub extern "C" fn vm_global(vm: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        let g = Value::object(slot.vm.global());
        slot.handle(g)
    })
}

/// A handle to a new ordinary object (0 if out of memory).
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_object(vm: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        match slot.vm.new_object() {
            Ok(o) => slot.handle(Value::object(o)),
            Err(_) => 0,
        }
    })
}

/// A handle to a new host function `fn_id` named staging[0..name_len].
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_host_function(vm: u32, fn_id: u32, name_len: usize) -> u32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let name = staged_str(staging, 0, name_len).to_owned();
        match slot.vm.new_host_function(fn_id, &name) {
            Ok(f) => slot.handle(f),
            Err(_) => 0,
        }
    })
}

/// A handle to a new async host function `fn_id` named staging[0..name_len] (D1374):
/// calling it returns a promise and adds a pending call (`vm_new_host_calls`).
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_async_host_function(vm: u32, fn_id: u32, name_len: usize) -> u32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let name = staged_str(staging, 0, name_len).to_owned();
        match slot.vm.new_async_host_function(fn_id, &name) {
            Ok(f) => slot.handle(f),
            Err(_) => 0,
        }
    })
}

/// Defines property staging[0..name_len] of object `obj` (a handle) as `value` (a handle),
/// writable and configurable but not enumerable, like builtins. Returns 0, or -1 if `obj`
/// is not an object or the heap is full.
#[unsafe(no_mangle)]
pub extern "C" fn vm_define(vm: u32, obj: u32, name_len: usize, value: u32) -> i32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let name = staged_str(staging, 0, name_len).to_owned();
        let Some(o) = slot.value(obj).as_object() else {
            return -1;
        };
        let v = slot.value(value);
        if slot.vm.define(o, &name, v).is_ok() {
            0
        } else {
            -1
        }
    })
}

/// Decodes the wire buffer staging[0..len] (docs/wire.md) into a handle. 0 on error, with
/// the reason in the out buffer.
#[unsafe(no_mangle)]
pub extern "C" fn vm_decode_wire(vm: u32, len: usize) -> u32 {
    with(|s| {
        // Decodes straight from staging (no copy); `out` is written after the borrow ends.
        let (slot, staging) = s.vm_and_staging(vm);
        let input = staged(staging, 0, len);
        let heap = slot.vm.heap_mut();
        let opts = DecodeOptions {
            object_proto: intrinsic(heap, Intrinsic::ObjectProto).as_object(),
            array_proto: intrinsic(heap, Intrinsic::ArrayProto).as_object(),
            ..DecodeOptions::default()
        };
        let msg = match wire::decode(heap, input, &opts) {
            Ok(v) => match slot.handle(v) {
                0 => "out of memory".to_string(),
                h => return h,
            },
            Err(e) => e.to_string(),
        };
        s.set_out(msg.as_bytes());
        0
    })
}

/// A handle to the value of the JSON text (UTF-8) in staging[0..len], parsed like
/// `JSON.parse` (objects and arrays get the realm's prototypes); 0 on failure (the
/// SyntaxError or "out of memory" in `out`). For hosts that already hold JSON text.
#[unsafe(no_mangle)]
pub extern "C" fn vm_parse_json(vm: u32, len: usize) -> u32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let input = staged(staging, 0, len);
        let heap = slot.vm.heap_mut();
        let opts = json::ParseOptions {
            object_proto: intrinsic(heap, Intrinsic::ObjectProto).as_object(),
            array_proto: intrinsic(heap, Intrinsic::ArrayProto).as_object(),
            ..json::ParseOptions::default()
        };
        let msg = match json::parse_utf8(heap, input, &opts) {
            Ok(v) => match slot.handle(v) {
                0 => "out of memory".to_string(),
                h => return h,
            },
            Err(json::ParseError::Syntax(e)) => e.to_string(),
            Err(json::ParseError::Heap(_)) => "out of memory".to_string(),
        };
        s.set_out(msg.as_bytes());
        0
    })
}

/// A handle to `JSON.stringify(value)` as a heap string, for plain data (objects, arrays,
/// primitives without getters, proxies or toJSON); 0 on failure with the reason in `out`
/// ("not plain data" for anything else, "undefined" when the result is undefined).
#[unsafe(no_mangle)]
pub extern "C" fn vm_json_stringify(vm: u32, h: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        let v = slot.value(h);
        let msg = match json::stringify_fast(slot.vm.heap_mut(), v, &[]) {
            Ok(Some(str)) => match slot.handle(Value::string(str)) {
                0 => "out of memory",
                h => return h,
            },
            Ok(None) => "undefined",
            Err(json::StringifyError::Slow) => "not plain data",
            Err(e) => e.js_error().map_or("error", |(_, m)| m),
        };
        s.set_out(msg.as_bytes());
        0
    })
}

/// A handle to a new Error of `kind` (0 Error, 1 TypeError, 2 RangeError, 3
/// ReferenceError, 4 SyntaxError, 5 InternalError) with message staging[0..msg_len].
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_error(vm: u32, kind: u32, msg_len: usize) -> u32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let msg = staged_str(staging, 0, msg_len).to_owned();
        let kind = match kind {
            1 => ErrorKind::TypeError,
            2 => ErrorKind::RangeError,
            3 => ErrorKind::ReferenceError,
            4 => ErrorKind::SyntaxError,
            5 => ErrorKind::InternalError,
            _ => ErrorKind::Error,
        };
        match slot.vm.new_error(kind, &msg) {
            Ok(e) => slot.handle(e),
            Err(_) => 0,
        }
    })
}

// ---- Values out ----------------------------------------------------------------------

/// Writes the 64 bits of a handle's value to the out buffer as two u32 (lo, hi), for
/// js/src/reader.ts.
#[unsafe(no_mangle)]
pub extern "C" fn vm_handle_bits(vm: u32, h: u32) {
    with(|s| {
        let bits = s.slot(vm).value(h).to_bits();
        s.set_out_words(&[bits as u32, (bits >> 32) as u32]);
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vm_handle_free(vm: u32, h: u32) {
    with(|s| {
        if h != 0 {
            s.slot(vm)
                .vm
                .heap_mut()
                .drop_handle(Handle::from_index(h - 1));
        }
    })
}

/// Formats a handle's value as `console.log` in `djs run` shows it (strings raw, Errors
/// as `Name: message`, the rest as JSON) into the out buffer (UTF-8); returns its length.
#[unsafe(no_mangle)]
pub extern "C" fn vm_display(vm: u32, h: u32) -> usize {
    with(|s| {
        let slot = s.slot(vm);
        let v = slot.value(h);
        let text = slot.vm.display(v);
        s.set_out(text.as_bytes());
        s.out.len()
    })
}

// ---- Calls and running ---------------------------------------------------------------

/// Starts calling `f` (a handle) with `this` (a handle, 0 = undefined) and `argc`
/// argument handles from staging (u32 LE each). Returns 0, or -1 if a call is already in
/// progress or the heap is full.
#[unsafe(no_mangle)]
pub extern "C" fn vm_call(vm: u32, f: u32, this: u32, argc: usize) -> i32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let args: Vec<Value> = staged(staging, 0, argc * 4)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| slot.value(u32::from_le_bytes(*c)))
            .collect();
        let (f, this) = (slot.value(f), slot.value(this));
        if slot.vm.call(f, this, &args).is_ok() {
            0
        } else {
            -1
        }
    })
}

/// Runs with `fuel`. Returns a [`status`] code. After DONE or THREW, `vm_result` hands
/// out the value; after HOST_CALL, `vm_host_call` describes the call.
#[unsafe(no_mangle)]
pub extern "C" fn vm_run(vm: u32, fuel: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        match slot.vm.run(fuel as u64) {
            Status::Done(v) => {
                slot.result = slot.handle(v);
                status::DONE
            }
            Status::Threw(v) => {
                slot.result = slot.handle(v);
                status::THREW
            }
            Status::OutOfFuel => status::OUT_OF_FUEL,
            Status::HostCall { call_id, .. } => {
                slot.call_id = call_id;
                status::HOST_CALL
            }
            Status::Idle => status::IDLE,
            // The call is abandoned; the VM is idle (D1690).
            Status::HostOutOfMemory => status::HOST_OUT_OF_MEMORY,
        }
    })
}

/// Sets the clock `Date.now()` and `new Date()` read, in ms since the epoch
/// (`Vm::set_now`, D1383). The VM reads no system clock: the host sets it, typically
/// before every `vm_run`. With `vm_resolve_host_call_at`, the only exports with an f64 argument.
#[unsafe(no_mangle)]
pub extern "C" fn vm_set_now(vm: u32, ms: f64) {
    with(|s| s.slot(vm).vm.set_now(ms))
}

/// What WeakRef and FinalizationRegistry do (`Vm::set_weak_refs`, D1680): 0 collected
/// (weak, the default), 1 strong (no collection is observable), 2 absent (the globals are
/// deleted). Set it on a new VM, before running code. Returns 0, or -1 for a bad mode or
/// a full heap.
#[unsafe(no_mangle)]
pub extern "C" fn vm_set_weak_refs(vm: u32, mode: u32) -> i32 {
    use djs_core::vm::WeakRefs;
    let mode = match mode {
        0 => WeakRefs::Collected,
        1 => WeakRefs::Strong,
        2 => WeakRefs::Absent,
        _ => return -1,
    };
    with(|s| {
        if s.slot(vm).vm.set_weak_refs(mode).is_ok() {
            0
        } else {
            -1
        }
    })
}

/// Reseeds `Math.random` with the 64-bit seed `hi << 32 | lo` (`Vm::set_random_seed`; 0
/// is the fixed default). A new VM starts with the default seed, so a host that wants
/// real randomness passes one from its own source; a replay passes a fixed one (0049).
#[unsafe(no_mangle)]
pub extern "C" fn vm_set_random_seed(vm: u32, lo: u32, hi: u32) {
    with(|s| {
        s.slot(vm)
            .vm
            .set_random_seed((u64::from(hi) << 32) | u64::from(lo))
    })
}

/// Sets the local time zone as a fixed offset in minutes east of UTC
/// (`Vm::set_time_zone_offset`; 0, UTC, by default).
#[unsafe(no_mangle)]
pub extern "C" fn vm_set_time_zone_offset(vm: u32, minutes: i32) {
    with(|s| s.slot(vm).vm.set_time_zone_offset(minutes))
}

/// Abandons the current call (after OUT_OF_FUEL, or while parked on a host call): the VM
/// is idle and usable again, its globals intact (`Vm::abort`).
#[unsafe(no_mangle)]
pub extern "C" fn vm_abort(vm: u32) {
    with(|s| {
        let slot = s.slot(vm);
        slot.vm.abort();
        slot.call_id = 0;
    })
}

/// The handle of the last DONE/THREW value (the host frees it). 0 if there is none.
#[unsafe(no_mangle)]
pub extern "C" fn vm_result(vm: u32) -> u32 {
    with(|s| std::mem::take(&mut s.slot(vm).result))
}

/// Describes the parked host call: writes `[call_id, fn_id, this_lo, this_hi, (arg_lo,
/// arg_hi) * argc]` as u32 to the out buffer and returns `argc`. The raw values are valid
/// until the next `vm_run` or collection.
#[unsafe(no_mangle)]
pub extern "C" fn vm_host_call(vm: u32) -> u32 {
    with(|s| {
        let id = s.slot(vm).call_id;
        s.describe_call(vm, id)
    })
}

/// `vm_host_call` for any pending call by id (an async one from `vm_new_host_calls`).
#[unsafe(no_mangle)]
pub extern "C" fn vm_host_call_info(vm: u32, call_id: u32) -> u32 {
    with(|s| s.describe_call(vm, call_id))
}

/// The async host calls made since the last call of this (D1374): writes their ids as
/// u32 to the out buffer and returns how many. Read them after every `vm_run`.
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_host_calls(vm: u32) -> u32 {
    with(|s| {
        let ids = s.slot(vm).vm.new_host_calls();
        s.set_out_words(&ids);
        ids.len() as u32
    })
}

/// The reasons of the promises rejected without a handler since the last call (D1373,
/// D1423): writes a handle (u32) per reason to the out buffer, oldest first, and returns
/// how many. The host frees them. Check after DONE, THREW or IDLE, as qjs does after its
/// job queue ran.
#[unsafe(no_mangle)]
pub extern "C" fn vm_take_unhandled_rejections(vm: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        let reasons = slot.vm.take_unhandled_rejections();
        let handles: Vec<u32> = reasons.into_iter().map(|r| slot.handle(r)).collect();
        s.set_out_words(&handles);
        handles.len() as u32
    })
}

/// The state of the promise `h` (a handle): 0 not a promise, 1 pending, 2 fulfilled, 3
/// rejected. When settled, the out buffer holds a handle (u32) to its result, which the
/// host frees, and the promise counts as handled (`Vm::mark_promise_handled`): the host
/// consumes its outcome, so a rejection is not also an unhandled one.
#[unsafe(no_mangle)]
pub extern "C" fn vm_promise_state(vm: u32, h: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        let v = slot.value(h);
        let (code, result) = match slot.vm.promise_state(v) {
            None => (0, None),
            Some(None) => (1, None),
            Some(Some(Ok(r))) => (2, Some(r)),
            Some(Some(Err(r))) => (3, Some(r)),
        };
        if result.is_some() {
            slot.vm.mark_promise_handled(v);
        }
        let words: Vec<u32> = result.map(|r| slot.handle(r)).into_iter().collect();
        s.set_out_words(&words);
        code
    })
}

/// Completes host call `call_id` with `value` (a handle, 0 = undefined): returned when
/// `ok` is 1, thrown when 0. Returns 0, or -1 for an unknown call. An async call's result
/// waits in the VM's inbox until the job queue is empty (D1670).
#[unsafe(no_mangle)]
pub extern "C" fn vm_resolve_host_call(vm: u32, call_id: u32, ok: u32, value: u32) -> i32 {
    resolve(vm, call_id, ok, value, None)
}

/// `vm_resolve_host_call` with the clock (`vm_set_now`'s ms) that `Date` reads from when
/// the result is applied (`Vm::resolve_host_call_at`, D1670).
#[unsafe(no_mangle)]
pub extern "C" fn vm_resolve_host_call_at(
    vm: u32,
    call_id: u32,
    ok: u32,
    value: u32,
    now: f64,
) -> i32 {
    resolve(vm, call_id, ok, value, Some(now))
}

fn resolve(vm: u32, call_id: u32, ok: u32, value: u32, now: Option<f64>) -> i32 {
    with(|s| {
        let slot = s.slot(vm);
        let v = slot.value(value);
        let r = if ok != 0 { Ok(v) } else { Err(v) };
        let done = match now {
            Some(ms) => slot.vm.resolve_host_call_at(call_id, r, ms),
            None => slot.vm.resolve_host_call(call_id, r),
        };
        if done.is_ok() { 0 } else { -1 }
    })
}

#[cfg(test)]
mod tests;
