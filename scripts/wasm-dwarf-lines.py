#!/usr/bin/env python3
"""Code bytes per source file (and line) from the DWARF line table of a wasm module (0052).

Unlike the name section, the line table attributes inlined code to the source it came
from, so this answers "which source makes Exec::run 56 KB" or "how much code comes from
core/alloc generics inlined into djs functions".

Build a module with line tables first (the code is the same as without them):
  CARGO_TARGET_DIR=work/scratch/dbg CARGO_PROFILE_WASM_DEBUG=1 \
    cargo build -p djs-wasm --target wasm32-unknown-unknown --profile wasm
then: llvm-dwarfdump --debug-line module.wasm > lines.txt (homebrew llvm has it)

usage: scripts/wasm-dwarf-lines.py module.wasm lines.txt [--func REGEX] [--lines] [--top N]
  --func REGEX  only functions whose demangled name matches (wasm-funcs.py naming)
  --lines       group by file:line instead of file
"""
import os
import re
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(__file__))


def leb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            return r, i


def function_ranges(path):
    """(start, end) of each function body, relative to the code section payload, which is
    how wasm DWARF addresses code."""
    b = open(path, "rb").read()
    i = 8
    while i < len(b):
        sid = b[i]
        size, start = leb(b, i + 1)
        if sid == 10:
            n, k = leb(b, start)
            out = []
            for _ in range(n):
                s, j = leb(b, k)
                out.append((k - start, j + s - start))
                k = j + s
            return out
        i = start + size
    return []


def parse_lines(path):
    """Rows (addr, file, line) per sequence, from llvm-dwarfdump --debug-line output."""
    dirs, files, seqs, cur = {}, {}, [], []
    fname = None
    for line in open(path):
        m = re.match(r"include_directories\[\s*(\d+)\] = \"(.*)\"", line)
        if m:
            dirs[int(m.group(1))] = m.group(2)
            continue
        if line.startswith("debug_line["):
            dirs, files = {}, {}
            continue
        m = re.match(r"file_names\[\s*(\d+)\]:", line)
        if m:
            fidx = int(m.group(1))
            continue
        m = re.match(r"\s+name: \"(.*)\"", line)
        if m:
            fname = m.group(1)
            continue
        m = re.match(r"\s+dir_index: (\d+)", line)
        if m:
            files[fidx] = os.path.join(dirs.get(int(m.group(1)), ""), fname)
            continue
        m = re.match(r"0x([0-9a-f]+)\s+(\d+)\s+(\d+)\s+(\d+)\s.*?(end_sequence)?\s*$", line)
        if m:
            addr = int(m.group(1), 16)
            cur.append((addr, files.get(int(m.group(4)), "?"), int(m.group(2))))
            if m.group(5):
                seqs.append(cur)
                cur = []
    return seqs


def short(f):
    f = re.sub(r".*/rustlib/src/rust/library/", "", f)
    f = re.sub(r".*/\.cargo/registry/src/[^/]+/", "", f)
    f = re.sub(r"^.*?crates/", "", f)
    return f


def main():
    args = sys.argv[1:]
    wasm, lines = args[0], args[1]
    func_re, by_line, top = None, False, 50
    rest = args[2:]
    while rest:
        a = rest.pop(0)
        if a == "--func":
            func_re = re.compile(rest.pop(0))
        elif a == "--lines":
            by_line = True
        elif a == "--top":
            top = int(rest.pop(0))
    ranges = function_ranges(wasm)
    keep = None
    if func_re:
        import importlib.util
        spec = importlib.util.spec_from_file_location(
            "wasm_funcs", os.path.join(os.path.dirname(__file__), "wasm-funcs.py"))
        wf = importlib.util.module_from_spec(spec)
        sys.argv = ["x"]
        src = open(spec.origin).read().replace("\nmain()\n", "\n")
        exec(compile(src, spec.origin, "exec"), wf.__dict__)
        names = [n for n, _ in wf.functions(wasm)]
        keep = [r for r, n in zip(ranges, names) if func_re.search(n)]
        print(f"{len(keep)} functions match, {sum(e - s for s, e in keep):,} bytes")

    def wanted(a):
        if keep is None:
            return True
        return any(s <= a < e for s, e in keep)

    agg = defaultdict(int)
    covered = 0
    for seq in parse_lines(lines):
        for (a, f, ln), (b, _, _) in zip(seq, seq[1:]):
            if b <= a or not wanted(a):
                continue
            key = f"{short(f)}:{ln}" if by_line else short(f)
            agg[key] += b - a
            covered += b - a
    print(f"{covered:,} bytes covered by the line table\n")
    for k, v in sorted(agg.items(), key=lambda kv: -kv[1])[:top]:
        print(f"{v:>9,} {100 * v / max(covered, 1):5.1f}%  {k}")


main()
