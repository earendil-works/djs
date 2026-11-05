#!/usr/bin/env python3
"""The gate's recognition of rustc incremental corruption in linker output (D2896, D3022 and
its amendment, 0231): link failures it retries after cleaning, and ones it must not.

    python3 scripts/gate-test.py
"""

import importlib.util
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location("gate", Path(__file__).with_name("gate.py"))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)

# ld64 (native, macOS): v0-mangled drop glue without an `.llvm.` suffix, referenced from one
# of djs_core's codegen units (pmem, 0234).
LD64 = """error: linking with `cc` failed: exit status: 1
  |
  = note: Undefined symbols for architecture arm64:
            "__RINvNtCs3bJEMBXHbwl_4core3ptr9drop_glueINtNtCs4lIPZ2R4bsq_5alloc3vec3VecNtNtCs1roge3bUHgV_8djs_core5value5ValueEECs1roge3bUHgV_8djs_core", referenced from:
                __RNvNtNtCs1roge3bUHgV_8djs_core8builtins5array15sort_primitives in djs_core-dada0b41a2376ff7.4k2vy5rgefunvkumahbpdghuy.1ikwzdm.rcgu.o
          ld: symbol(s) not found for architecture arm64
          clang: error: linker command failed with exit code 1 (use -v to see invocation)
"""

# rust-lld (wasm): LLVM-local anonymous symbols (0231's build:wasm).
RUST_LLD = """error: linking with `rust-lld` failed: exit status: 1
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: rust-lld: error: /g/target/wasm-dev/wasm32-unknown-unknown/debug/deps/libdjs_core-aea0b1040aeb1b05.rlib(djs_core-aea0b1040aeb1b05.4k2vy5rgefunvkumahbpdghuy.1ikwzdm.rcgu.o): undefined symbol: anon.90a37ed6ce647881c22d79c7aa3d490e.74.llvm.13953830260220837465
          rust-lld: error: /g/target/wasm-dev/wasm32-unknown-unknown/debug/deps/libdjs_core-aea0b1040aeb1b05.rlib(djs_core-aea0b1040aeb1b05.96b2nzzysgyai8kavhhdjhsio.1ikwzdm.rcgu.o): undefined symbol: anon.90a37ed6ce647881c22d79c7aa3d490e.13.llvm.13953830260220837465
"""

# rust-lld with a mangled Rust symbol and no suffix, from djs_prelude's object.
RUST_LLD_NO_SUFFIX = """  = note: rust-lld: error: /g/deps/libdjs_prelude-9f3192bcd2fd58f4.rlib(djs_prelude-9f3192bcd2fd58f4.abc123.1ikwzdm.rcgu.o): undefined symbol: _RNvMs_NtCs1roge3bUHgV_8djs_core4heapNtB4_4Heap6concat
"""

# GNU ld: the object on the line before.
GNU_LD = """  = note: /usr/bin/ld: /t/deps/djs_cli-0123456789abcdef.djs_cli.5f2a-cgu.3.rcgu.o: in function `djs_cli::main':
          main.rs:(.text+0x42): undefined reference to `_ZN8djs_core4heap4Heap3new17h0123456789abcdefE'
          collect2: error: ld returned 1 exit status
"""

# Not corruption: a C library's own undefined symbol, and a compile error.
FOREIGN = """  = note: /usr/bin/ld: /usr/lib/libfoo.a(foo.o): in function `foo_init':
          foo.c:(.text+0x10): undefined reference to `bar_open'
          collect2: error: ld returned 1 exit status
"""
COMPILE_ERROR = """error[E0425]: cannot find value `undefined_symbol` in this scope
 --> crates/djs-core/src/lib.rs:1:1
"""

failures = []
for name, text, want in [
    ("ld64 drop glue", LD64, True),
    ("rust-lld .llvm.", RUST_LLD, True),
    ("rust-lld no suffix", RUST_LLD_NO_SUFFIX, True),
    ("GNU ld", GNU_LD, True),
    ("foreign library", FOREIGN, False),
    ("compile error", COMPILE_ERROR, False),
]:
    got = gate.link_corruption(text)
    if got != want:
        failures.append(f"{name}: link_corruption is {got}, want {want}")
for f in failures:
    print("FAIL", f)
if failures:
    sys.exit(1)
print("gate self-test: ok (6 cases)")
