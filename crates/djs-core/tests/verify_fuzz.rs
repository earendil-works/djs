//! The image verifier against corrupted images (D1056, D2520): whatever the bytes are,
//! `Heap::from_bytes` returns Ok or Err and never panics or reads outside the arena. It is
//! what stands between an untrusted image and the accessors, which with `unsafe-heap` do
//! not check bounds (debug builds still assert them, so an out-of-range read by the
//! verifier fails this test in both builds). The gate runs it with and without the
//! feature.

use djs_core::heap::Heap;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

/// A heap with most cell kinds: the base (functions, templates, bytecode, shapes,
/// dictionaries, atoms) plus what a script leaves behind (strings of both widths, ropes,
/// arrays, maps, sets, closures, a promise, a typed array, a bigint, a proxy).
fn image() -> Vec<u8> {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let src = br#"
        globalThis.keep = [
          "latin1 string", "two-byte \u2603 string", "a".repeat(40) + "b".repeat(40),
          [1, 2.5, "x", null, undefined, true], { a: 1, b: { c: [3] } },
          new Map([[1, "one"], ["two", 2]]), new Set([1, 2, 3]), (x => y => x + y)(1),
          Promise.resolve(7), new Uint8Array([1, 2, 3]), 12345678901234567890n,
          new Proxy({}, {}), Symbol("s"), new WeakMap(), /a+b/g, new Date(0),
        ];
        let d = {}; for (let i = 0; i < 40; i++) d["k" + i] = i; delete d.k3; keep.push(d);
    "#;
    let f = vm.compile(src, &djs_core::compiler::CompileOptions::script("fuzz")).unwrap();
    let g = Value::object(vm.global());
    vm.call(f, g, &[]).unwrap();
    assert!(matches!(vm.run(10_000_000), Status::Done(_)));
    vm.collect().unwrap();
    vm.bytes().to_vec()
}

/// xorshift64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn verify_never_panics_on_corrupt_images() {
    let good = image();
    assert!(Heap::from_bytes(good.clone(), usize::MAX).is_ok());
    let n = good.len();
    // Cell starts, for plausible-looking corruption (a reference to another cell, a size
    // that lands on a neighbour).
    let heap = Heap::from_bytes(good.clone(), usize::MAX).unwrap();
    let mut starts = Vec::new();
    let mut pos = 0u32;
    while (pos as usize) < n {
        starts.push(pos);
        pos += heap.cell_size(pos);
    }
    drop(heap);
    let iterations = if cfg!(debug_assertions) { 3000 } else { 20000 };
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut accepted, mut rejected) = (0, 0);
    for _ in 0..iterations {
        let mut img = good.clone();
        for _ in 0..1 + rng.below(4) {
            // Aligned words are where headers, lengths and references live.
            let at = rng.below(n / 4) * 4;
            let word: u32 = match rng.below(6) {
                0 => rng.next() as u32,
                1 => starts[rng.below(starts.len())],
                2 => starts[rng.below(starts.len())] + 4 * rng.below(4) as u32,
                3 => rng.below(64) as u32,
                4 => u32::MAX - rng.below(16) as u32,
                _ => u32::from_le_bytes(img[at..at + 4].try_into().unwrap()) ^ (1 << rng.below(32)),
            };
            img[at..at + 4].copy_from_slice(&word.to_le_bytes());
        }
        match Heap::from_bytes(img, usize::MAX) {
            Ok(_) => accepted += 1,
            Err(_) => rejected += 1,
        }
    }
    eprintln!("verify fuzz: {n} bytes, {} cells, accepted {accepted}, rejected {rejected}", starts.len());
    // Both outcomes must occur, or the mutations test nothing.
    assert!(accepted > 0 && rejected > 0, "accepted {accepted}, rejected {rejected}");
    // Truncated images.
    for cut in [0, 7, 8, 64, n / 2, n - 8, n - 1] {
        let _ = Heap::from_bytes(good[..cut].to_vec(), usize::MAX);
    }
    // A VM image through the VM's loader too (the build hash and interpreter checks).
    let mut img = good.clone();
    let at = rng.below(n / 4) * 4;
    img[at] ^= 0x55;
    let _ = Vm::from_bytes(img, VmOptions::default());
}
