#!/usr/bin/env python3
"""Size experiment patch for scripts/size/variant.sh (0156): one execution path. The wasm
build drops the second ways to run the same thing: the `@native` state machines
(NATIVES empty, no native frames), the `@fast` Rust entries (FAST empty) and the tier-0
loop `run_hot` (every instruction through `step`). The base image is unchanged; the
builtins then run as bytecode. ONEPATH=native,fast,hot (default: all three) picks which.
Never merged: it only measures what each path costs in the module."""
import os
import sys

which = set(os.environ.get("ONEPATH", "native,fast,hot").split(","))


def sub(path, old, new):
    s = open(path).read()
    if old not in s:
        sys.exit(f"onepath.py: {old!r} not found in {path}")
    open(path, "w").write(s.replace(old, new, 1))


def empty_on_wasm(path, decl):
    sub(path, decl + " = &[",
        f'#[cfg(target_arch = "wasm32")]\n{decl} = &[];\n#[cfg(not(target_arch = "wasm32"))]\n{decl} = &[')


flags = []
if "native" in which:
    empty_on_wasm("crates/djs-core/src/builtins/gen/natives.rs", "pub(crate) static NATIVES: &[NativeFn]")
else:
    flags.append("func_flags::NATIVE")
if "fast" in which:
    empty_on_wasm("crates/djs-core/src/builtins/fast.rs", "pub(crate) static FAST: &[FastFn]")
else:
    flags.append("func_flags::FAST")
sub("crates/djs-core/src/interp/run.rs", "func_flags::NATIVE | func_flags::FAST",
    'if cfg!(target_arch = "wasm32") { %s } else { func_flags::NATIVE | func_flags::FAST }'
    % (" | ".join(flags) or "0"))
if "hot" in which:
    sub("crates/djs-core/src/interp/run.rs", "if self.opts.hot_loop {",
        'if self.opts.hot_loop && !cfg!(target_arch = "wasm32") {')
    sub("crates/djs-core/src/interp/run/native.rs", "&& self.opts.hot_loop",
        '&& self.opts.hot_loop && !cfg!(target_arch = "wasm32")')
