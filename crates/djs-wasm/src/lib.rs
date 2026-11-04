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
//! Panics trap (wasm32-unknown-unknown aborts on panic). The panic handler records a
//! message first, so the host can read it with `djs_panic_ptr` / `djs_panic_len`
//! afterwards. After a trap the instance must not be used again.
//!
//! On wasm32 the crate is `no_std` (D2332): its own panic handler never formats, so
//! neither std's panic machinery nor the panic messages and locations of the engine
//! reach the shipped module. Release modules report a fixed message; debug modules
//! (`debug_assertions`) and native builds keep the real one.

#![cfg_attr(target_arch = "wasm32", no_std)]

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;

mod allocator;

use djs_core::compiler::{CompileOptions, Goal};
use djs_core::heap::Handle;
use djs_core::interp::{ErrorKind, Intrinsic, intrinsic};
use djs_core::json;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};
use djs_core::wire::{self, DecodeOptions};

/// The ABI version, bumped on any incompatible change to the exports.
pub const ABI_VERSION: u32 = 3;

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
    /// The fuel the last `vm_run` used (`vm_fuel_used`).
    fuel_used: u64,
}

#[derive(Default)]
struct State {
    vms: Vec<Option<Slot>>,
    staging: Vec<u8>,
    out: Vec<u8>,
}

/// The instance's globals: thread-locals natively (each test thread has its own), plain
/// statics on wasm32, where an instance has one thread and no std.
macro_rules! instance_locals {
    ($($(#[$m:meta])* static $name:ident: $t:ty = $init:expr;)*) => {
        #[cfg(not(target_arch = "wasm32"))]
        std::thread_local! { $($(#[$m])* static $name: $t = const { $init };)* }
        $(
            $(#[$m])*
            #[cfg(target_arch = "wasm32")]
            static $name: wasm::Local<$t> = wasm::Local($init);
        )*
    };
}

instance_locals! {
    // `const`: no lazy-initialization code in each of the ~40 `with` instantiations
    // (0043: 10 KB of wasm).
    static STATE: RefCell<State> = RefCell::new(State {
        vms: Vec::new(),
        staging: Vec::new(),
        out: Vec::new(),
    });
    /// The base snapshot, decoded from the coded image at the first use (D2252) or written
    /// by the host (`djs_base_inject`). The first `vm_new` takes it as its
    /// arena, growing it in place to the arena's capacity (at memory's end, D2053). After
    /// that, a use decodes it again and the instance keeps it, for later VMs to copy
    /// (D2060).
    static BASE: RefCell<Option<Vec<u8>>> = RefCell::new(None);
    /// A VM took the base as its arena: from now on the instance keeps a decoded base.
    static BASE_TAKEN: core::cell::Cell<bool> = core::cell::Cell::new(false);
    /// A copy of the base the host is writing (`djs_base_inject_ptr`), until checked.
    static INJECTED: RefCell<Option<Vec<u8>>> = RefCell::new(None);
    /// The coded base the host handed over (`djs_base_coded_ptr`, the module's `djs_base`
    /// section; D2541), kept while a later decode may need it.
    static CODED: RefCell<Option<Vec<u8>>> = RefCell::new(None);
    /// The message of the last panic. Separate from `STATE`, which may be left borrowed by
    /// the call that trapped.
    static PANIC: RefCell<String> = RefCell::new(String::new());
}

/// Records the message of a panic for `djs_panic_ptr` (best effort: never panics).
fn record_panic(msg: impl FnOnce() -> String) {
    PANIC.with(|p| {
        if let Ok(mut p) = p.try_borrow_mut() {
            *p = msg();
        }
    });
}

/// The wasm32 runtime without std (D2332): instance globals and the panic handler (the
/// allocator is in allocator.rs).
#[cfg(target_arch = "wasm32")]
mod wasm {
    /// A global of the instance. wasm32-unknown-unknown without the atomics feature has
    /// one thread, so a plain static is enough.
    pub struct Local<T>(pub T);

    // SAFETY: there is only one thread in a wasm32-unknown-unknown instance (no atomics,
    // no shared memory), so no value is ever reached from two threads.
    unsafe impl<T> Sync for Local<T> {}

    impl<T> Local<T> {
        #[inline]
        pub fn with<R>(&'static self, f: impl FnOnce(&T) -> R) -> R {
            f(&self.0)
        }
    }

    /// Never formats in release modules: formatting would keep core::fmt, every panic
    /// message and every panic location (source paths) in the module (D2332). Debug
    /// modules keep the message, as the node tests and hosts debugging the engine want.
    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        #[cfg(debug_assertions)]
        super::record_panic(|| alloc::string::ToString::to_string(info));
        #[cfg(not(debug_assertions))]
        {
            let _ = info;
            super::record_panic(|| {
                alloc::string::String::from(
                    "djs-wasm panicked (an engine bug; the release module has no panic \
                     messages, a debug or native build has them)",
                )
            });
        }
        core::arch::wasm32::unreachable()
    }
}

/// A buffer with the capacity a VM's arena starts with (`Heap::restore_capacity` at the
/// default cap), for the base to be decoded into when a VM needs it at once.
fn base_buffer() -> Vec<u8> {
    Vec::with_capacity(base_capacity())
}

/// [`base_buffer`]'s capacity.
fn base_capacity() -> usize {
    djs_core::heap::Heap::image_capacity(djs_prelude::base_len(), djs_core::heap::DEFAULT_LIMIT)
}

/// The base snapshot (D1321), decoded now unless the instance holds it (D2252), and its
/// address (valid until the next `vm_new` or `djs_base_inject_ptr`).
fn base_ptr() -> usize {
    if BASE.with(|b| b.borrow().is_none()) {
        // Exactly the base's size: an instance that only reads it (to restore a snapshot
        // against it) does not hold the arena's room.
        let Some(base) = decode_base(page_block(djs_prelude::base_len())) else {
            return 0;
        };
        BASE.with(|b| *b.borrow_mut() = Some(base));
    }
    BASE.with(|b| b.borrow().as_ref().map_or(0, |b| b.as_ptr() as usize))
}

/// The base decoded into a buffer of `capacity`: natively from djs-prelude's copy, in wasm
/// from the coded base the host handed over. `None` without one.
fn decode_base(capacity: usize) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    if CODED.with(|c| c.borrow().is_none()) {
        return djs_prelude::unpack_base(djs_prelude::BASE_PACKED, capacity);
    }
    CODED.with(|c| djs_prelude::unpack_base(c.borrow().as_deref()?, capacity))
}

/// At least the page allocator's huge-block size: a base smaller than that would live in
/// dlmalloc's memory, and when a VM takes it as its arena and reserves room, it moves
/// to pages and leaves its old block in dlmalloc's segment for good (0151).
fn page_block(len: usize) -> usize {
    #[cfg(target_arch = "wasm32")]
    return len.max(allocator::pages::HUGE);
    #[cfg(not(target_arch = "wasm32"))]
    len
}

/// The base snapshot as a buffer for a new VM's arena (D2060). The first VM of an
/// instance takes the instance's bytes (decoded or injected; decoded straight into the
/// arena's buffer if there are none). Later VMs copy the base, which the instance then
/// keeps: a host that makes several VMs in one instance decodes twice, not each time.
fn take_base() -> Option<Vec<u8>> {
    if !BASE_TAKEN.with(|t| t.get()) {
        let base = BASE
            .with(|b| b.borrow_mut().take())
            .or_else(|| decode_base(base_capacity()))?;
        BASE_TAKEN.with(|t| t.set(true));
        return Some(base);
    }
    if base_ptr() == 0 {
        return None;
    }
    // The instance keeps the decoded base now: the coded one is no longer needed.
    CODED.with(|c| c.borrow_mut().take());
    BASE.with(|b| {
        let b = b.borrow();
        let mut v = base_buffer();
        v.extend_from_slice(b.as_ref()?);
        Some(v)
    })
}

/// A VM from the base snapshot, or the reason there is none.
fn new_from_base(opts: VmOptions, live: u32) -> Result<Vm, &'static str> {
    let base = take_base().ok_or(
        "no base snapshot: the host must hand over the module's djs_base section \
         (djs_base_coded_ptr) or inject a decoded base",
    )?;
    djs_prelude::new_vm_owned(base, opts, live as usize).map_err(|e| e.as_str())
}

/// Natively, a panic hook records the message (and the default hook still prints). On
/// wasm32 the panic handler does that.
fn init() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let msg = std::string::ToString::to_string(info);
                record_panic(|| msg);
                previous(info);
            }));
        });
    }
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
        for v in core::iter::once(info.this).chain(info.args.iter().copied()) {
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
            fuel_used: 0,
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
    core::str::from_utf8(staged(staging, start, len))
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

// ---- Allocation statistics (feature `alloc-stats`) -------------------------------------

#[cfg(feature = "alloc-stats")]
mod alloc_stats {
    use core::alloc::{GlobalAlloc, Layout};
    use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

    // The allocator underneath (allocator.rs).
    use crate::allocator::Base as System;

    pub static LIVE: AtomicUsize = AtomicUsize::new(0);
    pub static PEAK: AtomicUsize = AtomicUsize::new(0);
    pub static COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static LARGEST: AtomicUsize = AtomicUsize::new(0);

    pub struct Counting;

    fn grew(by: usize) {
        let live = LIVE.fetch_add(by, Relaxed) + by;
        PEAK.fetch_max(live, Relaxed);
    }

    // SAFETY: forwards every call to the system allocator unchanged; only counts.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            COUNT.fetch_add(1, Relaxed);
            LARGEST.fetch_max(l.size(), Relaxed);
            grew(l.size());
            // SAFETY: the caller's contract is passed on.
            unsafe { System.alloc(l) }
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            LIVE.fetch_sub(l.size(), Relaxed);
            // SAFETY: as above.
            unsafe { System.dealloc(p, l) }
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            COUNT.fetch_add(1, Relaxed);
            LARGEST.fetch_max(n, Relaxed);
            if n >= l.size() {
                grew(n - l.size());
            } else {
                LIVE.fetch_sub(l.size() - n, Relaxed);
            }
            // SAFETY: as above.
            unsafe { System.realloc(p, l, n) }
        }
    }

    #[global_allocator]
    static A: Counting = Counting;
}

/// Allocation statistics: the address of four u32, valid until the next call: live
/// bytes, peak live bytes, allocation count, the largest single allocation. `reset`
/// nonzero restarts the peak at the live size and the count and largest at 0. Leaves the
/// out buffer alone.
#[cfg(feature = "alloc-stats")]
#[unsafe(no_mangle)]
pub extern "C" fn djs_alloc_stats(reset: u32) -> usize {
    use alloc_stats::*;
    use core::sync::atomic::{AtomicU32, Ordering::Relaxed};
    static OUT: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
    let v = [
        LIVE.load(Relaxed),
        PEAK.load(Relaxed),
        COUNT.load(Relaxed),
        LARGEST.load(Relaxed),
    ];
    if reset != 0 {
        PEAK.store(v[0], Relaxed);
        COUNT.store(0, Relaxed);
        LARGEST.store(0, Relaxed);
    }
    for (o, x) in OUT.iter().zip(v) {
        o.store(x as u32, Relaxed);
    }
    OUT.as_ptr() as usize
}

// ---- Buffers -------------------------------------------------------------------------

/// The ABI version ([`ABI_VERSION`]).
#[unsafe(no_mangle)]
pub extern "C" fn djs_abi_version() -> u32 {
    ABI_VERSION
}

/// The engine build hash (`djs_core::vm::BUILD_HASH`): it changes with the engine's
/// sources, cargo features and libm version, so a host can tell builds apart (journals,
/// pi's deterministic fingerprint, 0049; djs-prelude's base check). Images are keyed on
/// their dependency descriptor instead (D2265).
#[unsafe(no_mangle)]
pub extern "C" fn djs_build_hash() -> u32 {
    djs_core::vm::BUILD_HASH
}

/// The address of the base snapshot (D1321, decoded at the first use, D2252) in this
/// module's memory, read-only. Hosts compress snapshots against it (D2260): it is the
/// image every `vm_new` starts from, and the build hash fixes its bytes. Valid until the
/// next `vm_new`, which takes the bytes as its arena (D2060): a host that keeps the base
/// reads it first (a later call decodes it again). 0 if the instance has no base and no
/// coded base to decode (`djs_base_coded_ptr`).
#[unsafe(no_mangle)]
pub extern "C" fn djs_base_ptr() -> usize {
    base_ptr()
}

/// Room for the base snapshot a host decoded before (in another instance of this
/// module): the host writes its bytes there and calls `djs_base_inject`, and the instance
/// never decodes its own (D2252). 0 if this instance already has its base or `len` is not
/// the base's length.
#[unsafe(no_mangle)]
pub extern "C" fn djs_base_inject_ptr(len: usize) -> usize {
    if BASE.with(|b| b.borrow().is_some()) || len != djs_prelude::base_len() {
        return 0;
    }
    // The next `vm_new` takes it as its arena (D2060): with the arena's capacity, or
    // reserving that moves it, and a base below the page allocator's `HUGE` leaves its
    // first block behind in dlmalloc's memory (0151).
    let mut buf = base_buffer();
    buf.resize(len, 0);
    let p = buf.as_mut_ptr() as usize;
    INJECTED.with(|i| *i.borrow_mut() = Some(buf));
    p
}

/// Takes the bytes written at `djs_base_inject_ptr` as this instance's base if they are
/// the base (its length and FNV-1a hash): 1, or 0 (refused: the instance decodes its own
/// at the first use).
#[unsafe(no_mangle)]
pub extern "C" fn djs_base_inject() -> u32 {
    let Some(buf) = INJECTED.with(|i| i.borrow_mut().take()) else {
        return 0;
    };
    if !djs_prelude::is_base(&buf) {
        return 0;
    }
    BASE.with(|b| {
        let mut b = b.borrow_mut();
        if b.is_some() {
            return 0;
        }
        *b = Some(buf);
        1
    })
}

/// Room for the coded base, the module's custom section `djs_base` (D2541): the module
/// does not keep it in its memory, so the host hands it over when it has no decoded base
/// to inject. The host writes the section's `len` bytes there; the instance decodes them
/// when it needs the base (checking them) and drops them once it keeps a decoded copy. 0
/// if the instance has its base or the coded one, or `len` is not the section's length.
#[unsafe(no_mangle)]
pub extern "C" fn djs_base_coded_ptr(len: usize) -> usize {
    if BASE.with(|b| b.borrow().is_some())
        || CODED.with(|c| c.borrow().is_some())
        || len != djs_prelude::PACKED_LEN
    {
        return 0;
    }
    CODED.with(|c| {
        let mut buf = alloc::vec![0; len];
        let p = buf.as_mut_ptr() as usize;
        *c.borrow_mut() = Some(buf);
        p
    })
}

/// The length of the base snapshot in bytes (see `djs_base_ptr`).
#[unsafe(no_mangle)]
pub extern "C" fn djs_base_len() -> usize {
    djs_prelude::base_len()
}

/// Like `djs_staging`, for a VM image about to be restored with `vm_from_bytes` with a heap
/// cap of `memory_limit`: the buffer gets the capacity the restored arena will reserve, so
/// handing it over grows nothing. Restoring through `djs_staging` copies the arena once
/// more and leaves the staging block behind as a hole (D1942).
#[unsafe(no_mangle)]
pub extern "C" fn djs_image_staging(len: usize, memory_limit: u32) -> usize {
    let cap = djs_core::heap::Heap::image_capacity(len, options(memory_limit).memory_limit);
    with(|s| {
        if s.staging.capacity() < cap {
            // Free the old buffer first, so the new one can take its place.
            s.staging = Vec::new();
            s.staging.reserve_exact(cap);
        }
        s.staging.clear();
        s.staging.resize(len, 0);
        s.staging.as_mut_ptr() as usize
    })
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
    add_new_vm(new_from_base(options(memory_limit), 0))
}

/// Like `vm_new`, with the arena capacity a heap of `live` bytes gets (its next GC
/// threshold and slack) from the start: for a host that knows how big the heap will be
/// after its setup (pi's cold start runs its prelude next). Growing later would move the
/// arena and leave its old block behind as a hole in wasm memory.
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_reserved(memory_limit: u32, live: u32) -> u32 {
    add_new_vm(new_from_base(options(memory_limit), live))
}

/// `vm_new_with`'s flag: frozen builtins (`VmOptions::frozen_builtins`, D2304).
pub const VM_FROZEN_BUILTINS: u32 = 1;

/// Like `vm_new_reserved`, with creation options in `flags`: [`VM_FROZEN_BUILTINS`].
/// Returns 0 with the reason in the out buffer for an unknown flag.
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_with(memory_limit: u32, live: u32, flags: u32) -> u32 {
    if flags & !VM_FROZEN_BUILTINS != 0 {
        return with(|s| {
            s.set_out(b"unknown vm_new_with flags");
            0
        });
    }
    let opts = VmOptions {
        frozen_builtins: flags & VM_FROZEN_BUILTINS != 0,
        ..options(memory_limit)
    };
    add_new_vm(new_from_base(opts, live))
}

/// Whether the builtins of `vm` are frozen (`vm_new_with`, D2304): 1 or 0.
#[unsafe(no_mangle)]
pub extern "C" fn vm_frozen_builtins(vm: u32) -> u32 {
    with(|s| s.slot(vm).vm.frozen_builtins() as u32)
}

/// Like `vm_new`, but builds the realm and compiles the prelude instead of copying the
/// base (about 150x slower). For checking the base against the realm built in wasm; only
/// with the `compiled-prelude` feature.
#[cfg(feature = "compiled-prelude")]
#[unsafe(no_mangle)]
pub extern "C" fn vm_new_compiled(memory_limit: u32) -> u32 {
    // Made into an image like djs-prelude's build makes the base, so that the two can be
    // compared byte for byte.
    let vm = Vm::new(options(memory_limit)).and_then(|mut vm| {
        vm.prepare_base_image()?;
        Ok(vm)
    });
    add_new_vm(vm.map_err(|e| e.as_str()))
}

fn add_new_vm(vm: Result<Vm, &str>) -> u32 {
    with(|s| match vm {
        Ok(vm) => s.add_vm(vm),
        Err(e) => {
            s.set_out(&[b"cannot create a VM: ", e.as_bytes()].concat());
            0
        }
    })
}

/// A VM restored from an image (`vm_arena_*` bytes of a module with the same dependency
/// descriptor) in staging[0..len]: images made in this process only (the dump module's
/// transport, pi's prelude images). They are not a persistence format (D2700), so only the
/// header and the descriptor are checked, not every cell (`Vm::from_bytes_trusted`).
/// Returns its number, or 0 with the reason in the out buffer.
#[unsafe(no_mangle)]
pub extern "C" fn vm_from_bytes(len: usize, memory_limit: u32) -> u32 {
    with(|s| {
        staged(&s.staging, 0, len);
        // The image becomes the arena: hand over the staging buffer instead of copying.
        let mut bytes = core::mem::take(&mut s.staging);
        bytes.truncate(len);
        match Vm::from_bytes_trusted(bytes, options(memory_limit)) {
            Ok(vm) => s.add_vm(vm),
            Err(e) => {
                s.set_out(&[b"cannot restore the VM image: ", e.text().as_bytes()].concat());
                0
            }
        }
    })
}

/// Replaces VM `vm`'s state by the image in staging[0..len] (as `vm_from_bytes`, an image
/// made in this process), keeping what the VM derived from the base on an earlier dump
/// (`Vm::replace_image`, D2701): a host that dumps through a dump module keeps one VM
/// there for all its dumps. Handles of the old state are gone. 0: done; 1: refused, the
/// reason in out (the VM keeps its old state).
#[unsafe(no_mangle)]
pub extern "C" fn vm_replace_image(vm: u32, len: usize) -> i32 {
    with(|s| {
        staged(&s.staging, 0, len);
        let mut bytes = core::mem::take(&mut s.staging);
        bytes.truncate(len);
        let slot = s.slot(vm);
        match slot.vm.replace_image(bytes) {
            Ok(()) => {
                slot.result = 0;
                slot.call_id = 0;
                0
            }
            Err(e) => {
                s.set_out(&[b"cannot restore the VM image: ", e.text().as_bytes()].concat());
                1
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

/// Collects garbage and makes everything live old (D2041): later collections while
/// scripts run neither trace nor move it. For the end of the embedder's setup (pi's
/// prelude). Returns 0, or -1 if the heap could not reserve the side tables.
#[unsafe(no_mangle)]
pub extern "C" fn vm_seal(vm: u32) -> i32 {
    with(|s| if s.slot(vm).vm.seal().is_ok() { 0 } else { -1 })
}

// ---- Code ----------------------------------------------------------------------------

/// Compiles staging[params_len..params_len + src_len] (UTF-8 source). `goal` 0 is a
/// Script; 1 is a codemode async function body whose parameter names are
/// staging[0..params_len], separated by `\n`. `goal | 2` parses TypeScript (0048), `| 4`
/// with legacy `experimentalDecorators`, `| 8` also with `emitDecoratorMetadata` (0065).
/// Returns a handle to the closure, or 0 with the error (`SyntaxError: ... at
/// line:col`) in the out buffer. Stack traces name the code `sandbox.js`;
/// `vm_compile_file` names it.
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
            experimental_decorators: goal & 12 != 0,
            emit_decorator_metadata: goal & 8 != 0,
        };
        let result = slot.vm.compile(&src, &opts).map_err(|e| e.text());
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

/// The bytecode listing of a script, as `djs disasm` prints it, for the docs site's live
/// examples (0113). Staging holds the parameter names (`\n`-separated, `params_len`
/// bytes), then the source; `goal` as for `vm_compile`. Compiles into a scratch heap, not a
/// VM. Returns 1 with the listing in the out buffer, or 0 with the compile error. Only
/// with the `docs` feature, so the shipping module does not carry the disassembler.
#[cfg(feature = "docs")]
#[unsafe(no_mangle)]
pub extern "C" fn djs_disasm(goal: u32, params_len: usize, src_len: usize) -> u32 {
    use djs_core::bytecode::template::disasm_template;
    with(|s| {
        let params_text = staged_str(&s.staging, 0, params_len).to_owned();
        let src = staged(&s.staging, params_len, src_len).to_vec();
        let params: Vec<&str> = params_text.split('\n').filter(|p| !p.is_empty()).collect();
        let opts = CompileOptions {
            goal: if goal & 1 == 0 {
                Goal::Script
            } else {
                Goal::AsyncBody
            },
            filename: "example.js",
            params: &params,
            privileged: false,
            typescript: goal & 2 != 0,
            experimental_decorators: goal & 12 != 0,
            emit_decorator_metadata: goal & 8 != 0,
        };
        let listing = djs_core::heap::Heap::new()
            .map_err(|e| String::from(e.as_str()))
            .and_then(|mut heap| {
                djs_core::compiler::compile(&mut heap, &src, &opts)
                    .map(|t| disasm_template(&heap, t))
                    .map_err(|e| e.text())
            });
        match listing {
            Ok(text) => {
                s.set_out(text.as_bytes());
                1
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

/// Writes the idle VM as a format 2 dump (`Vm::dump`, 0167, 0172; docs/functions-as-code.md,
/// docs/continuation-dumps.md) to the out buffer: its global bindings, and what is in
/// flight (suspended activations, open host calls, pending promises). staging[0..len] is
/// UTF-8, four lines (0179): the globals the host defines (re-bound by name, never
/// written) and the names to dump (none: all of the VM's own), each comma-separated; the
/// host's function names in id order, tab-separated (`DumpOptions::host_fn_name`: open
/// calls and host functions held as values are written by name); the values to name for
/// the host, `handle:name`, tab-separated (`DumpOptions::handles`, written as
/// `$dump.handle(name, v)`; handle 0: the entry call's outcome, `Vm::outcome`, D2633);
/// the host's states of open calls (D2770), `call_id:byte_len`, tab-separated, and after
/// that line's newline the states' JSON texts one after another (`DumpOptions::call_states`,
/// written as `$dump.pending`'s fourth argument). 0: the dump is in out; 1: refused, the
/// reasons in out (one per line); 2: no base image (inject it first).
#[cfg(feature = "dump")]
#[unsafe(no_mangle)]
pub extern "C" fn vm_dump(vm: u32, len: usize) -> i32 {
    if base_ptr() == 0 {
        return 2;
    }
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let text = staged_str(staging, 0, len).to_owned();
        // Five lines, then the states' text, which may hold newlines of its own.
        let mut lines = text.splitn(6, '\n');
        let mut line = || lines.next().unwrap_or("");
        let host: Vec<&str> = line().split(',').filter(|n| !n.is_empty()).collect();
        let names: Vec<&str> = line().split(',').filter(|n| !n.is_empty()).collect();
        let fns: Vec<&str> = line().split('\t').collect();
        let host_fn_name = |id: u32| {
            let name = fns.get((id as usize).checked_sub(1)?)?;
            (!name.is_empty()).then(|| String::from(*name))
        };
        let handles: Vec<(&str, Value)> = line()
            .split('\t')
            .filter_map(|e| {
                let (h, name) = e.split_once(':')?;
                let v = match h.parse().ok()? {
                    // The entry call's outcome, without making a handle for it.
                    0 => match slot.vm.outcome()? {
                        Ok(v) | Err(v) => v,
                    },
                    h => slot.value(h),
                };
                Some((name, v))
            })
            .collect();
        let state_lens = line();
        let state_text = line();
        let mut call_states: Vec<(u32, &str)> = Vec::new();
        let mut at = 0;
        for e in state_lens.split('\t').filter(|e| !e.is_empty()) {
            let state = e.split_once(':').and_then(|(id, n)| {
                let end = at + n.parse::<usize>().ok()?;
                let json = state_text.get(at..end)?;
                at = end;
                Some((id.parse().ok()?, json))
            });
            match state {
                Some(s) => call_states.push(s),
                None => {
                    s.set_out(b"bad call states in staging\n");
                    return 1;
                }
            }
        }
        let r = BASE.with(|b| {
            let b = b.borrow();
            let base = b.as_deref()?;
            let opts = djs_core::vm::dump::DumpOptions {
                base,
                host: &host,
                host_fn_name: &host_fn_name,
                handles: &handles,
                call_states: &call_states,
            };
            Some(
                slot.vm
                    .dump(&opts, (!names.is_empty()).then_some(names.as_slice())),
            )
        });
        match r {
            Some(Ok(t)) => {
                s.set_out(t.as_bytes());
                0
            }
            Some(Err(e)) => {
                let mut lines = String::new();
                for r in &e.0 {
                    lines.push_str(r);
                    lines.push('\n');
                }
                s.set_out(lines.as_bytes());
                1
            }
            None => 2,
        }
    })
}

/// Loads a dump into the VM with `Vm::load_dump` (0170, D2581): version 1 as a plain
/// Script, format 2 with the loader's `$dump`. staging[0..names_len] names the host's
/// functions, `\n`-separated, the i-th (from 0) having id i + 1: the loader maps an open
/// call's name (`$dump.pending(id, "sleep", args)`) to its id with it (0179). The dump's
/// UTF-8 text follows, `len` bytes. 0: loaded, out = what the load returned (`Loaded`):
/// u32 words `n_calls, n_handles, (call_id, fn_id, state_len) * n_calls, (handle,
/// name_len) * n_handles`, then the handles' names, UTF-8, one after another, then the
/// calls' states (D2770; JSON text, `state_len` bytes, 0: the call has none). The handles
/// are the host's to free. 1: not loaded, the reason in out.
#[cfg(feature = "dump")]
#[unsafe(no_mangle)]
pub extern "C" fn vm_load_dump(vm: u32, names_len: usize, len: usize) -> i32 {
    with(|s| {
        let (slot, staging) = s.vm_and_staging(vm);
        let names = staged_str(staging, 0, names_len).to_owned();
        let text = staged_str(staging, names_len, len).to_owned();
        let host_fn = |name: &str| {
            names
                .split('\n')
                .position(|n| !n.is_empty() && n == name)
                .map(|i| i as u32 + 1)
        };
        let opts = djs_core::vm::dump_load::LoadOptions {
            host_fn: &host_fn,
            ..Default::default()
        };
        match slot.vm.load_dump(&text, &opts) {
            Ok(loaded) => {
                let mut words = vec![loaded.calls.len() as u32, loaded.handles.len() as u32];
                let mut text = Vec::new();
                for c in &loaded.calls {
                    let state = c.state.as_deref().unwrap_or("");
                    words.extend([c.id, c.fn_id, state.len() as u32]);
                }
                for (name, h) in &loaded.handles {
                    words.extend([h.index() + 1, name.len() as u32]);
                    text.extend_from_slice(name.as_bytes());
                }
                for c in &loaded.calls {
                    text.extend_from_slice(c.state.as_deref().unwrap_or("").as_bytes());
                }
                s.set_out_words(&words);
                s.out.extend_from_slice(&text);
                0
            }
            Err(e) => {
                s.set_out(e.message().as_bytes());
                1
            }
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
                0 => String::from("out of memory"),
                h => return h,
            },
            Err(e) => e.text(),
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
                0 => String::from("out of memory"),
                h => return h,
            },
            Err(json::ParseError::Syntax(e)) => e.text(),
            Err(json::ParseError::Heap(_)) => String::from("out of memory"),
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

/// A handle to a new one-byte string of `len` zero code units for the host to fill in
/// place (D2551), or 0 if the heap is full or `len` too long. Its data is the `len` bytes
/// at `vm_arena_ptr` + the string's offset (`vm_handle_bits`' low word) + 16, one
/// Latin-1 unit per byte (an ASCII string's UTF-8 as is); the host fills them before its
/// next call into the module. A large host string so skips the staging copy and the
/// transcode.
#[unsafe(no_mangle)]
pub extern "C" fn vm_string_reserve(vm: u32, len: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        match slot.vm.reserve_string(len) {
            Ok(str) => slot.handle(Value::string(str)),
            Err(_) => 0,
        }
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
        let (st, used) = slot.vm.run_metered(fuel as u64);
        slot.fuel_used = used;
        match st {
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

/// The fuel the last `vm_run` used (saturated to u32): a host that stops at host calls
/// counts its fuel across them, so a loop of sync host calls still reaches its interrupt
/// polls (D2653). It can exceed the fuel given by the cost of the instruction that ran out.
#[unsafe(no_mangle)]
pub extern "C" fn vm_fuel_used(vm: u32) -> u32 {
    with(|s| u32::try_from(s.slot(vm).fuel_used).unwrap_or(u32::MAX))
}

/// Sets the clock `Date.now()` and `new Date()` read, in ms since the epoch
/// (`Vm::set_now`, D1383). The VM reads no system clock: the host sets it, typically
/// before every `vm_run`. With `vm_resolve_host_call_at`, the only exports with an f64 argument.
#[unsafe(no_mangle)]
pub extern "C" fn vm_set_now(vm: u32, ms: f64) {
    with(|s| s.slot(vm).vm.set_now(ms))
}

/// The VM's canonical hash (`Vm::canonical_hash`, D1730): FNV-1a 64 of the image a
/// collection would leave, computed on a copy (the VM is unchanged). Writes it to the out
/// buffer as u32 lo, hi; returns 0, or -1 when there is no memory for the copy.
#[unsafe(no_mangle)]
pub extern "C" fn vm_canonical_hash(vm: u32) -> i32 {
    with(|s| match s.slot(vm).vm.canonical_hash() {
        Ok(h) => {
            s.set_out_words(&[h as u32, (h >> 32) as u32]);
            0
        }
        Err(_) => -1,
    })
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

/// Reseeds `Math.random` alone with `hi << 32 | lo`, keeping the hash key
/// (`Vm::set_math_random_seed`, D2433).
#[unsafe(no_mangle)]
pub extern "C" fn vm_set_math_random_seed(vm: u32, lo: u32, hi: u32) {
    with(|s| {
        s.slot(vm)
            .vm
            .set_math_random_seed((u64::from(hi) << 32) | u64::from(lo))
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
    with(|s| core::mem::take(&mut s.slot(vm).result))
}

/// A new handle (the host frees it) to the entry call's outcome while no frame is active
/// (`Vm::outcome`): also at IDLE, which is where a host that calls a VM function while
/// async host calls are pending gets its return value (pi's adapter in its `next` mode,
/// D2650). Unlike `vm_result` it can be taken any number of times. 0 if there is none (a
/// frame is active, or nothing was called); out[0] = 1 if the call threw, else 0.
#[unsafe(no_mangle)]
pub extern "C" fn vm_outcome(vm: u32) -> u32 {
    with(|s| {
        let slot = s.slot(vm);
        let Some(r) = slot.vm.outcome() else {
            s.set_out_words(&[0]);
            return 0;
        };
        let (v, threw) = match r {
            Ok(v) => (v, 0),
            Err(e) => (e, 1),
        };
        let h = slot.handle(v);
        s.set_out_words(&[threw]);
        h
    })
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
