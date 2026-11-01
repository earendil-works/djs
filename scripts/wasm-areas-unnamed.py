#!/usr/bin/env python3
"""Approximate code size per area of a wasm module without a name section, such as
quickjs-wasi's quickjs.wasm (0052). Only for rough comparisons: the labels are guesses.

Each function is labeled from what it references: export names, C strings (assert
messages carry the source file and __func__, error messages are distinctive), matched
by the SEEDS regexes below (first match wins). Unlabeled functions then take the
majority area of their callers, repeated until stable, then of their callees. The
"seeded" column says how many bytes were labeled directly.

usage: scripts/wasm-areas-unnamed.py module.wasm [--top N]   (needs wasm-objdump)
"""
import re
import subprocess
import sys
from collections import Counter, defaultdict


def leb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            return r, i


def sleb(b, i):
    r = shift = 0
    while True:
        x = b[i]
        i += 1
        r |= (x & 0x7F) << shift
        shift += 7
        if x < 0x80:
            if x & 0x40:
                r -= 1 << shift
            return r, i


def read_module(path):
    """(memory segments [(addr, bytes)], code body sizes, imported function count)."""
    b = open(path, "rb").read()
    i, segs, sizes, imports = 8, [], [], 0
    while i < len(b):
        sid = b[i]
        size, j = leb(b, i + 1)
        if sid == 2:
            n, k = leb(b, j)
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
            n, k = leb(b, j)
            for _ in range(n):
                s, m = leb(b, k)
                sizes.append(s + (m - k))
                k = m + s
        elif sid == 11:
            n, k = leb(b, j)
            for _ in range(n):
                flags, k = leb(b, k)
                if flags != 0:
                    raise SystemExit("only active segments with an i32.const offset")
                addr, k = sleb(b, k + 1)  # i32.const
                k += 1  # end
                sz, k = leb(b, k)
                segs.append((addr, b[k:k + sz]))
                k += sz
        i = j + size
    return segs, sizes, imports


def main():
    args = sys.argv[1:]
    top = 40
    if "--top" in args:
        top = int(args[args.index("--top") + 1])
    path = args[0]
    segs, body_sizes, imports = read_module(path)

    def cstr(addr):
        for base, data in segs:
            if base <= addr < base + len(data):
                off = addr - base
                end = data.find(b"\0", off)
                if end < 0 or end - off < 3:
                    return None
                s = data[off:end]
                return s.decode() if all(32 <= c < 127 for c in s) else None
        return None

    sizes = {imports + k: s for k, s in enumerate(body_sizes)}
    exports = {}
    for line in subprocess.run(["wasm-objdump", "-x", "-j", "Export", path], capture_output=True,
                               text=True).stdout.splitlines():
        m = re.search(r"func\[(\d+)\] <([^>]*)>", line)
        if m:
            exports[int(m.group(1))] = m.group(2)
    funcs, cur = {}, None
    dis = subprocess.run(["wasm-objdump", "-d", path], capture_output=True, text=True).stdout
    for line in dis.splitlines():
        m = re.match(r"^[0-9a-f]+ func\[(\d+)\]", line)
        if m:
            cur = int(m.group(1))
            funcs[cur] = {"strs": [], "calls": []}
            continue
        if cur is None:
            continue
        m = re.search(r"i32.const (\d+)$", line)
        if m:
            s = cstr(int(m.group(1)))
            if s and s not in funcs[cur]["strs"]:
                funcs[cur]["strs"].append(s)
            continue
        m = re.search(r"\bcall (\d+)", line)
        if m:
            funcs[cur]["calls"].append(int(m.group(1)))

    SEEDS = [
        ("interpreter: JS_CallInternal", r"^invalid opcode: pc="),
        ("libc/printf/malloc", r"^(memcpy|memmove|memset|memcmp|strchr|strstr|strlen|sscanf|bsearch|qsort|snprintf|vsnprintf|wasm_malloc|malloc|free|realloc|calloc|abort|strcmp|strncmp)$|^-\+   0X0x|^\(null\)$|^Success$|^DNn$"),
        ("libm", r"^(pow|ceil|floor|log|log2|log10|log1p|exp|expm1|sin|cos|tan|asin|acos|atan|atan2|sinh|cosh|tanh|asinh|acosh|atanh|cbrt|hypot|fmod|remainder|round|trunc|sqrt|fma|lrint|rint|nearbyint)f?$"),
        ("regexp", r"libregexp\.c|regular expression|regexp|^lre_|^re_|group name|quantifier|class range|invalid escape sequence in regular|back reference|imbricated|lookbehind|RegExp"),
        ("unicode tables/case", r"libunicode\.c|unicode property|^cr_|^unicode_|case_conv|normaliz"),
        ("number parse/format", r"dtoa\.c|js_dtoa|js_atod|toPrecision|toFixed|toExponential|radix|^Infinity$|^NaN$"),
        ("bigint", r"BigInt|js_bigint"),
        ("json", r"JSON|^Bad control character|Unexpected end of JSON|after property name|Bad Unicode escape"),
        ("date", r"Invalid Date|Date value|^GMT$|SunMonTue|JanFebMar"),
        ("compiler (parser + codegen)", r"^js_parse|^emit_|^resolve_|^optimize_|^skip_|^push_scope|^add_(var|arg|scope)|^ident_realloc|^next_token|^js_create_function|^code_match|unexpected token|expecting|syntax error|strict mode|label|not in a function|pass 1|parenthes|destructuring|lexical declaration|formal parameter|line terminator|class statement|invalid (method|field) name|redefinition|unexpected (end|character|'|ellipsis)|identifier expected|Octal|number literal|template|getter or setter|fd->|s->cur_func|OP_label|bc_buf|closure_var|__JS_EvalInternal|super|new\.target|yield|await|duplicate (label|parameter|__proto__)|use strict"),
        ("snapshot/serialization", r"JS_ReadObject|JS_WriteObject|bc_read|bc_write|<bad tag>|no bytecode allowed|unsupported object class|only value properties|unsupported symbol type|invalid object reference|read after the end"),
        ("debug dumps/leak check", r"JS_DumpMemoryUsage|JS_ComputeMemoryUsage|leak|JSAtom hash table|JSShapes|%-15s|%5d:|JS_FreeRuntime"),
        ("modules", r"module|import\.meta|JS_MODULE_STATUS|export name|exported variable"),
        ("gc/runtime", r"mark_children|gc_|free_(function|object|var_ref)|^js_free|ref_count|JS_FreeContext|compact_properties|shape"),
        ("promise/async/generator", r"promise|Promise|generator|already running|async"),
        ("typed arrays/arraybuffer", r"ArrayBuffer|typed array|TypedArray|DataView|detached|byteOffset|invalid offset|invalid length"),
        ("proxy/reflect", r"[Pp]roxy|revoked"),
        ("builtins (other)", r"not an object|not a function|not a constructor|Symbol|Object|Array|String|Function|Error|Iterator|Map|Set|WeakRef|FinalizationRegistry|cannot convert|invalid array length|toString|prototype|^js_|^JS_"),
    ]

    label, how = {}, {}
    for i, v in funcs.items():
        texts = ([exports[i]] if i in exports else []) + v["strs"]
        for area, pat in SEEDS:
            if any(re.search(pat, t) for t in texts):
                label[i], how[i] = area, "seed"
                break
    callers = defaultdict(list)
    for i, v in funcs.items():
        for c in v["calls"]:
            callers[c].append(i)

    def propagate(edges_of, tag):
        changed = True
        while changed:
            changed = False
            for i in funcs:
                if i in label:
                    continue
                votes = Counter(label[j] for j in edges_of(i) if j in label)
                if votes:
                    label[i], how[i] = votes.most_common(1)[0][0], tag
                    changed = True

    propagate(lambda i: callers[i], "callers")
    propagate(lambda i: funcs[i]["calls"], "callees")

    total = sum(sizes.values())
    agg = defaultdict(lambda: [0, 0, 0])
    for i, sz in sizes.items():
        a = label.get(i, "unlabeled")
        agg[a][0] += sz
        agg[a][1] += 1
        if how.get(i) == "seed":
            agg[a][2] += sz
    print(f"{path}: code {total:,} bytes in {len(sizes)} functions (approximate areas)\n")
    print("    bytes      %  funcs  seeded-bytes  area")
    for a, (sz, n, seeded) in sorted(agg.items(), key=lambda kv: -kv[1][0])[:top]:
        print(f"{sz:>9,} {100 * sz / total:5.1f}% {n:>6} {seeded:>13,}  {a}")


main()
