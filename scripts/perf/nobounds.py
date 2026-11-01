#!/usr/bin/env python3
"""Experiment (scratch copy only, never merged): arena field reads/writes without bounds
checks (layout.rs read_array/write_array use get_unchecked), to measure what the checks
cost. Builds exp/nobounds/src with LTO, cgu=1 into exp/nobounds/target."""
import os, subprocess, sys
tools = os.path.dirname(os.path.abspath(__file__))
root = os.path.abspath(os.path.join(tools, "../.."))
# Builds and outputs (scripts/perf/README.md): $DJS_PERF_DIR, default work/scratch/perf.
here = os.environ.get("DJS_PERF_DIR", os.path.join(root, "work/scratch/perf"))
os.makedirs(here, exist_ok=True)
dst = os.path.join(here, "nobounds/src")
subprocess.run(["rsync", "-a", "--delete", "--exclude", "target", "--exclude", "work", "--exclude", "target-lto", "--exclude", "vendor",
                "--exclude", "node_modules", "--exclude", ".git", root + "/", dst + "/"], check=True)
p = os.path.join(dst, "crates/djs-core/src/heap/layout.rs")
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
lib = os.path.join(dst, "crates/djs-core/src/lib.rs")
l = open(lib).read().replace("#![forbid(unsafe_code)]", "").replace("#![deny(unsafe_code)]", "")
open(lib, "w").write(l)
env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(here, "nobounds/target"), CARGO_PROFILE_RELEASE_LTO="fat", CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1")
subprocess.run(["cargo", "build", "--release", "-p", "djs-cli"], cwd=dst, env=env, check=True)
subprocess.run(["cargo", "build", "-p", "djs-wasm", "--target", "wasm32-unknown-unknown", "--profile", "wasm"], cwd=dst, env=dict(env, CARGO_PROFILE_WASM_STRIP="debuginfo"), check=True)
