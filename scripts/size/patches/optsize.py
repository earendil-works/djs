#!/usr/bin/env python3
"""Size experiment patch for scripts/size/variant.sh (0156): `#[optimize(size)]` (LLVM
minsize, opt-level "z" per function) on every function of the djs-core directories in
OPTSIZE (default: the cold ones: compiler, lexer, bigint, wire, unicode, plus the regexp
parser and compiler; OPTSIZE=all for the whole crate, which matches a module-wide
opt-level "z"). Uses the nightly `optimize_attribute` feature, which the wasm build's
RUSTC_BOOTSTRAP (D2333) allows. Never merged."""
import os
import pathlib
import re
import sys

src = pathlib.Path("crates/djs-core/src")
lib = src / "lib.rs"
lib.write_text(lib.read_text().replace("#![no_std]", "#![no_std]\n#![feature(optimize_attribute)]", 1))
dirs = os.environ.get("OPTSIZE", "compiler,lexer,bigint,wire,unicode").split(",")
files = list(src.rglob("*.rs")) if dirs == ["all"] else [f for d in dirs for f in (src / d).rglob("*.rs")]
if dirs != ["all"] and "OPTSIZE" not in os.environ:
    files += [src / "regexp/compiler.rs", src / "regexp/parser.rs"]
fn = re.compile(r"^(\s*)(pub(\([\w:]+\))? )?(const )?fn \w+")
n = 0
for p in files:
    if "test" in p.name:
        continue
    out = []
    for line in p.read_text().split("\n"):
        m = fn.match(line)
        # Not on bodiless trait method declarations.
        if m and not line.rstrip().endswith(";"):
            out.append(m.group(1) + "#[optimize(size)]")
            n += 1
        out.append(line)
    p.write_text("\n".join(out))
print(f"optsize.py: {n} functions", file=sys.stderr)
