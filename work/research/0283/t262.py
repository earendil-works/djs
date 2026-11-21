#!/usr/bin/env python3
"""t262.py: active test262 tests per builtin family: a test counts for a family if it lies
in one of the family's directories or declares one of its features (front matter), or, for
syntax families, uses the syntax (a regex on the source)."""
import re, sys, collections
T = "/Users/mitsuhiko/Development/djs/vendor/test262/test/"
paths = [l.split(" ")[0] for l in open(sys.argv[1]) if l.strip()]
FAM = {
    "typed arrays, ArrayBuffer, DataView": (
        ["built-ins/TypedArray", "built-ins/TypedArrayConstructors", "built-ins/ArrayBuffer", "built-ins/DataView",
         "built-ins/Atomics", "built-ins/SharedArrayBuffer", "built-ins/Uint8Array"],
        {"TypedArray", "ArrayBuffer", "DataView", "Float16Array", "resizable-arraybuffer", "arraybuffer-transfer",
         "immutable-arraybuffer", "uint8array-base64", "SharedArrayBuffer", "Atomics", "TypedArray.prototype.at",
         "stable-typedarray-sort"} , r"\b(Int8|Uint8|Uint8Clamped|Int16|Uint16|Int32|Uint32|Float16|Float32|Float64|BigInt64|BigUint64)Array\b|\bArrayBuffer\b|\bDataView\b"),
    "Iterator helpers": (["built-ins/Iterator"], {"iterator-helpers", "iterator-sequencing", "iterator-chunking",
                         "iterator-includes", "Iterator.prototype.join"}, None),
    "Proxy": (["built-ins/Proxy"], {"Proxy", "proxy-missing-checks"}, r"\bnew Proxy\b|\bProxy\.revocable\b"),
    "Reflect": (["built-ins/Reflect"], {"Reflect", "Reflect.construct", "Reflect.set", "Reflect.setPrototypeOf"}, r"\bReflect\."),
    "DisposableStack, using": (["built-ins/DisposableStack", "built-ins/AsyncDisposableStack", "built-ins/SuppressedError"],
                               {"explicit-resource-management"}, None),
    "decorators": ([], {"decorators"}, None),
    "WeakMap, WeakSet": (["built-ins/WeakMap", "built-ins/WeakSet"], {"WeakMap", "WeakSet", "symbols-as-weakmap-keys"}, r"\bnew Weak(Map|Set)\b"),
    "WeakRef, FinalizationRegistry": (["built-ins/WeakRef", "built-ins/FinalizationRegistry"], {"WeakRef", "FinalizationRegistry"}, None),
    "Array.fromAsync": (["built-ins/Array/fromAsync"], {"Array.fromAsync"}, None),
    "BigInt": (["built-ins/BigInt"], {"BigInt"}, r"\b\d+n\b|\bBigInt\("),
    "URI functions": (["built-ins/decodeURI", "built-ins/decodeURIComponent", "built-ins/encodeURI",
                       "built-ins/encodeURIComponent", "annexB/built-ins/escape", "annexB/built-ins/unescape"], set(), None),
    "atob/btoa, DOMException": ([], set(), r"\b(atob|btoa|DOMException)\b"),
}
fm_re = re.compile(r"/\*---(.*?)---\*/", re.S)
def features(src):
    m = fm_re.search(src)
    if not m: return set()
    f = re.search(r"features:\s*\[([^\]]*)\]", m.group(1))
    if f: return {x.strip() for x in f.group(1).split(",") if x.strip()}
    f = re.search(r"features:\s*\n((?:\s*-\s*.*\n)+)", m.group(1))
    return {x.strip()[1:].strip() for x in f.group(1).splitlines()} if f else set()
counts = collections.Counter(); per = collections.defaultdict(set)
for p in paths:
    try: src = open(T + p, encoding="utf-8", errors="replace").read()
    except FileNotFoundError: continue
    feats = features(src)
    body = fm_re.sub("", src)
    for fam, (dirs, fs, rx) in FAM.items():
        if any(p.startswith(d + "/") or p.startswith(d + ".") for d in dirs) or feats & fs or (rx and re.search(rx, body)):
            per[fam].add(p)
allfam = set().union(*per.values())
print(f"active tests: {len(paths)}")
for fam in FAM:
    print(f"{fam:34} {len(per[fam]):6}")
print(f"{'any of them':34} {len(allfam):6}")
import json; json.dump({k: sorted(v) for k, v in per.items()}, open("t262-per.json", "w"))
