#!/usr/bin/env python3
"""Size experiment patch for scripts/size/variant.sh (0123): arena field reads/writes
without bounds checks, as scripts/perf/nobounds.py does for instruction counts. Never
merged: it needs unsafe."""
import os
p = os.path.join(".", "crates/djs-core/src/heap/layout.rs")
s = open(p).read()
old_r = '''    match bytes.get(off..).and_then(<[u8]>::first_chunk::<N>) {
        Some(a) => *a,
        None => out_of_bounds(off, N, bytes.len()),
    }'''
new_r = '''    let _ = out_of_bounds;
    #[allow(unsafe_code)]
    unsafe { (bytes.as_ptr().add(off) as *const [u8; N]).read_unaligned() }'''
old_w = '''    let len = bytes.len();
    match bytes.get_mut(off..).and_then(<[u8]>::first_chunk_mut::<N>) {
        Some(a) => *a = v,
        None => out_of_bounds(off, N, len),
    }'''
new_w = '''    #[allow(unsafe_code)]
    unsafe { (bytes.as_mut_ptr().add(off) as *mut [u8; N]).write_unaligned(v) }'''
assert s.count(old_r) == 1 and s.count(old_w) == 1
s = s.replace(old_r, new_r).replace(old_w, new_w)
open(p, "w").write(s)
lib = os.path.join(".", "crates/djs-core/src/lib.rs")
l = open(lib).read().replace("#![forbid(unsafe_code)]", "").replace("#![deny(unsafe_code)]", "")
open(lib, "w").write(l)
