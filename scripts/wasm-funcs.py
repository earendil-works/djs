#!/usr/bin/env python3
"""Per-function code size of a wasm module, for size investigations (0052).

usage: scripts/wasm-funcs.py [module.wasm] [--areas] [--generic] [--top N]
  default: one `size<TAB>name` line per function (demangled with rustfilt when present),
  largest first.
  --areas    sum code by functional area (parser, interpreter, builtins, ...): see AREAS.
  --generic  group functions by their name without generic arguments, to find code that
             is monomorphized many times (count > 1).
The module needs its name section: `scripts/build-wasm.sh --keep-names`.
"""
import re
import shutil
import subprocess
import sys
from collections import defaultdict


def leb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            return r, i


def functions(path):
    b = open(path, "rb").read()
    i, imports, bodies, names = 8, 0, [], {}
    while i < len(b):
        sid = b[i]
        size, start = leb(b, i + 1)
        end = start + size
        if sid == 2:
            n, k = leb(b, start)
            for _ in range(n):
                for _ in range(2):
                    l, k = leb(b, k)
                    k += l
                kind = b[k]
                k += 1
                _, k = leb(b, k)
                if kind == 0:
                    imports += 1
        elif sid == 10:
            n, k = leb(b, start)
            for _ in range(n):
                s, j = leb(b, k)
                bodies.append(s + (j - k))
                k = j + s
        elif sid == 0:
            n, k = leb(b, start)
            if b[k:k + n] == b"name":
                k += n
                while k < end:
                    sub = b[k]
                    s, j = leb(b, k + 1)
                    if sub == 1:
                        count, m = leb(b, j)
                        for _ in range(count):
                            idx, m = leb(b, m)
                            l, m = leb(b, m)
                            names[idx] = b[m:m + l].decode(errors="replace")
                            m += l
                    k = j + s
        i = end
    mangled = [names.get(imports + k, f"func[{imports + k}]") for k in range(len(bodies))]
    if shutil.which("rustfilt"):
        out = subprocess.run(["rustfilt"], input="\n".join(mangled), capture_output=True,
                             text=True).stdout.split("\n")[:len(mangled)]
    else:
        out = mangled
    return list(zip(out, bodies))


# Functional areas, first match wins. Patterns are matched against the demangled name.
AREAS = [
    ("typescript (compiler/ts)", r"compiler::parser::Parser>::ts_|compiler::ts::"),
    ("regexp", r"djs_core::regexp::|builtins::regexp|json_regexp_intrinsic"),
    ("json", r"djs_core::json::"),
    ("bigint", r"djs_core::bigint::|builtins::bigint"),
    ("date", r"builtins::date"),
    ("number parse/format", r"djs_core::num::"),
    ("unicode tables/case", r"djs_core::unicode::"),
    ("lexer", r"djs_core::lexer::"),
    ("compiler (parser + codegen)", r"djs_core::compiler::|djs_core::bytecode::(code|template)"),
    ("interpreter: Exec::run", r"interp::run::Exec>::run$"),
    ("interpreter: call_intrinsic", r"interp::run::Exec>::(call_intrinsic|fast_intrinsic)$"),
    ("interpreter: rest", r"djs_core::interp::|djs_core::bytecode::"),
    ("builtins (Rust leaves)", r"djs_core::builtins::"),
    ("heap/gc/objects", r"djs_core::heap::"),
    ("snapshot/wire", r"djs_core::(wire|image|snapshot)::"),
    ("vm api", r"djs_core::vm::|djs_wasm::|^vm_|std::thread::local"),
    ("djs_core other", r"djs_core::"),
    ("libm", r"^(djs_)?libm::|compiler_builtins::math"),
    ("malloc", r"dlmalloc|__rdl_|__rust_(alloc|dealloc|realloc)"),
    ("core::fmt / panic", r"core::fmt|core::panicking|panic|core::result::unwrap_failed|core::option::(expect|unwrap)_failed"),
    ("alloc/core generics", r"^<?(alloc|core)::|^<\[|^<&|^<\(|^<(u8|u16|u32|usize|i64|char|bool|str)"),
    ("compiler_builtins", r"compiler_builtins|^__"),
]


def area_of(name):
    for area, pat in AREAS:
        if re.search(pat, name):
            return area
    return "other"


def generic_key(name):
    # Drop generic arguments (nested <...> after a path component) but keep impl brackets.
    s = re.sub(r"::<.*>$", "", name)
    s = re.sub(r"\s*\{closure#\d+\}", "{closure}", s)
    out, depth = [], 0
    for ch in s:
        if ch == "<":
            depth += 1
            out.append(ch if depth == 1 else "")
        elif ch == ">":
            out.append(ch if depth == 1 else "")
            depth -= 1
        elif depth <= 1:
            out.append(ch)
    s = "".join(out)
    m = re.match(r"^<(.*?) as (.*?)>::(.*)$", s)
    if m:
        # `<X as Trait>::f`: the instantiations differ in X, group by trait method
        s = f"<_ as {m.group(2)}>::{m.group(3)}"
    return s


def main():
    args = sys.argv[1:]
    path = "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm"
    mode, top = "list", 60
    while args:
        a = args.pop(0)
        if a == "--areas":
            mode = "areas"
        elif a == "--generic":
            mode = "generic"
        elif a == "--top":
            top = int(args.pop(0))
        else:
            path = a
    funcs = functions(path)
    total = sum(s for _, s in funcs)
    if mode == "list":
        for name, size in sorted(funcs, key=lambda f: -f[1]):
            print(f"{size}\t{name}")
        return
    groups = defaultdict(lambda: [0, 0])
    for name, size in funcs:
        key = area_of(name) if mode == "areas" else generic_key(name)
        groups[key][0] += size
        groups[key][1] += 1
    rows = sorted(groups.items(), key=lambda kv: -kv[1][0])
    if mode == "generic":
        rows = [r for r in rows if r[1][1] > 1]
    print(f"{path}: code {total:,} bytes in {len(funcs)} functions\n")
    for key, (size, count) in rows[:top]:
        print(f"{size:>9,} {100 * size / total:5.1f}% {count:>5}  {key[:150]}")


main()
