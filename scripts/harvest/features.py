#!/usr/bin/env python3
"""Which JavaScript features do real codemode scripts use? (0130)

Compiles every codemode execution of extract.py's output with djs's own compiler
(`djs disasm --body`, the codemode goal: the body of an async function with the
parameters `tools` and `console`) and reads the features off the bytecode: the opcodes,
the intrinsics they call, the global names they read and the property names they read,
write and call. A few things the bytecode does not show (labels, `new.target` written
by hand, sloppy-mode syntax in scripts djs rejects) come from jstok's token stream.

    scripts/harvest/features.py EXTRACT.jsonl OUT.jsonl [--djs target/debug/djs] [-j 6]
    scripts/harvest/features.py --summary OUT.jsonl [--prelude PRELUDE.js]

The first form writes one line per execution: its source id, session, time, the feature
keys and djs's compile error, if any. The second prints per-feature counts (executions,
distinct scripts, sessions, first and last date) as a markdown table. Session data is
private: OUT stays in work/scratch/ (gitignored) and holds no script text.
"""
import collections
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import jstok  # noqa: E402

# The globals of djs's realm (Object.getOwnPropertyNames(globalThis)), plus what QuickJS
# has and djs does not.
BUILTIN_GLOBALS = set(
    """globalThis NaN Infinity undefined Array ArrayBuffer BigInt Boolean DataView Date
    DisposableStack AsyncDisposableStack Error EvalError RangeError ReferenceError SyntaxError
    TypeError URIError InternalError AggregateError SuppressedError eval queueMicrotask Function
    Iterator JSON Map Math Number parseFloat parseInt isNaN isFinite Object Promise Proxy Reflect
    RegExp Set String Symbol Int8Array Uint8Array Uint8ClampedArray Int16Array Uint16Array
    Int32Array Uint32Array Float16Array Float32Array Float64Array BigInt64Array BigUint64Array
    decodeURI decodeURIComponent encodeURI encodeURIComponent escape unescape WeakMap WeakSet
    WeakRef FinalizationRegistry DOMException btoa atob console""".split()
)
QJS_ONLY_GLOBALS = {"SharedArrayBuffer", "Atomics", "performance", "navigator", "print", "scriptArgs", "gc"}
# What pi's codemode prelude and host define (prelude-source.ts, execute.ts).
CODEMODE_GLOBALS = {
    "tools", "ALL_TOOLS", "console", "text", "image", "exit", "store", "load", "models",
    "searchTools", "describeTool", "describeNamespace",
}
TYPED_ARRAYS = {
    "Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array", "Int32Array",
    "Uint32Array", "Float16Array", "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
}

ITER_FIELDS = {"next", "done", "value", "return", "throw"}
CORE_WORDS = {"try", "catch", "finally", "throw", "switch", "while", "do", "var", "let", "function", "new", "class"}
CORE_PUNCT = {
    "=>": "arrow-function", "?.": "optional-chaining", "??": "nullish-coalescing",
    "??=": "logical-assignment", "||=": "logical-assignment", "&&=": "logical-assignment",
    "...": "spread-or-rest", "**": "exponent", "?": "conditional",
}
OP_RE = re.compile(r"^\s+\d+\s+(\S+)(?:\s+(.*?))?(?:\s+;\s*\d+:\d+)?$")
FUNC_RE = re.compile(r"^function (\S+) (.*): params=")


def rust_unescape(s):
    """The text of a Rust `{:?}`-formatted string (without the quotes)."""
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        n = s[i + 1]
        if n == "u":
            j = s.index("}", i)
            out.append(chr(int(s[i + 3:j], 16)))
            i = j + 1
            continue
        out.append({"n": "\n", "t": "\t", "r": "\r", "0": "\0"}.get(n, n))
        i += 2
    return "".join(out)


def const_string(arg):
    """`#3 ("text")` → text, or None for a non-string constant."""
    m = re.match(r'#\d+ \("(.*)"\)$', arg or "")
    return rust_unescape(m.group(1)) if m else None


def regex_features(pattern, flags):
    """Features of a regular expression: flags and the syntax its pattern uses."""
    feats = {f"regex-flag:{f}" for f in flags}
    i = 0
    in_class = False
    while i < len(pattern):
        c = pattern[i]
        if c == "\\":
            n = pattern[i + 1:i + 2]
            if n in ("p", "P") and pattern[i + 2:i + 3] == "{":
                feats.add("regex:unicode-property")
            elif n == "k" and pattern[i + 2:i + 3] == "<":
                feats.add("regex:named-backref")
            elif n.isdigit() and n != "0" and not in_class:
                feats.add("regex:backref")
            elif n == "u" and pattern[i + 2:i + 3] == "{":
                feats.add("regex:codepoint-escape")
            elif n == "c":
                feats.add("regex:control-escape")
            i += 2
            continue
        if in_class:
            if c == "]":
                in_class = False
            elif c == "[" and "v" in flags:
                feats.add("regex:class-set")
            elif pattern.startswith("--", i) or pattern.startswith("&&", i):
                if "v" in flags:
                    feats.add("regex:class-set")
            i += 1
            continue
        if c == "[":
            in_class = True
        elif pattern.startswith("(?<=", i) or pattern.startswith("(?<!", i):
            feats.add("regex:lookbehind")
        elif pattern.startswith("(?=", i) or pattern.startswith("(?!", i):
            feats.add("regex:lookahead")
        elif pattern.startswith("(?<", i):
            feats.add("regex:named-group")
        elif re.match(r"\(\?[imsx-]+:", pattern[i:]):
            feats.add("regex:modifiers")
        elif c in "*+?}" and pattern[i + 1:i + 2] == "?" and i > 0:
            feats.add("regex:lazy")
        i += 1
    return feats


def parse_disasm(text):
    """[(function id, name, [(op, arg)])]."""
    funcs = []
    for line in text.splitlines():
        m = FUNC_RE.match(line)
        if m:
            funcs.append((m.group(1), m.group(2), []))
            continue
        m = OP_RE.match(line)
        if m and funcs:
            funcs[-1][2].append((m.group(1), m.group(2) or ""))
    return funcs


def atom(arg):
    m = re.match(r'"(.*)"(?: ic\d+)?$', arg)
    return rust_unescape(m.group(1)) if m else None


def bytecode_features(funcs, dotted=frozenset()):
    """`dotted`: the names the source writes after a `.` (tells the script's own
    `.next`/`.value` from the ones for-of and destructuring read)."""
    f = set()
    declared_globals = set()
    for fid, name, ops in funcs:
        if fid != "0":
            if name.startswith("get ") or name.startswith("set "):
                f.add("syntax:accessor")
        opnames = {op for op, _ in ops}
        if "generator_start" in opnames:
            f.add("syntax:async-generator" if "await" in opnames else "syntax:generator")
        if fid != "0" and "async_start" in opnames:
            f.add("syntax:async-function")
        for i, (op, arg) in enumerate(ops):
            prev = ops[i - 1] if i else ("", "")
            nxt = ops[i + 1] if i + 1 < len(ops) else ("", "")
            if op == "define_class":
                f.add("syntax:class")
            elif op == "call_intrinsic" and arg.startswith("%ClassHeritage"):
                f.add("syntax:class-extends")
            elif op in ("private_symbol", "get_private_field", "private_in"):
                f.add("syntax:class-private")
            elif op in ("define_class_field", "define_private_field"):
                f.add("syntax:class-field")
            elif op in ("get_super", "super_call", "super_call_spread"):
                f.add("syntax:super")
            elif op == "set_proto":
                f.add("syntax:__proto__-literal")
            elif op == "special_object" and arg == "0":
                f.add("syntax:arguments")
            elif op == "special_object" and arg == "4" and fid == "0":
                f.add("sloppy:top-level-this")
            elif op == "get_template_object":
                if prev[0] == "get_method" and atom(prev[1]) == "raw":
                    f.add("builtin:String.raw")
                else:
                    f.add("syntax:tagged-template")
            elif op == "call_intrinsic" and arg.startswith("%RegExpCreate"):
                f.add("syntax:regex-literal")
                pat, flags = const_string(ops[i - 2][1]), const_string(ops[i - 1][1])
                if pat is not None and flags is not None:
                    f |= regex_features(pat, flags)
            elif op == "push_const" and "(bigint@" in arg:
                f.add("syntax:bigint-literal")
            elif op == "pow":
                f.add("syntax:exponent")
            elif op == "delete":
                f.add("syntax:delete")
            elif op == "in":
                f.add("syntax:in")
            elif op == "instanceof":
                f.add("syntax:instanceof")
            elif op in ("typeof", "get_global_typeof"):
                f.add("syntax:typeof")
            elif op == "for_in_start":
                f.add("syntax:for-in")
            elif op == "call_intrinsic" and arg.startswith("%GetAsyncIterator"):
                f.add("syntax:for-await")
            elif op == "call_intrinsic" and arg.startswith("%GetIterator"):
                f.add("syntax:iteration (for-of, array destructuring)")
            elif op in ("call_spread", "construct_spread", "array_spread"):
                f.add("syntax:spread")
            elif op == "rest":
                f.add("syntax:rest-parameter")
            elif op == "call_intrinsic" and arg.startswith("%CopyDataPropertiesExcluding"):
                f.add("syntax:object-rest")
            elif op == "copy_data_props":
                f.add("syntax:object-spread")
            elif op == "yield_star":
                f.add("syntax:yield*")
            elif op == "debugger":
                f.add("syntax:debugger")
            elif op in ("define_getter", "define_setter", "define_getter_elem", "define_setter_elem"):
                f.add("syntax:accessor")
            elif op == "call_intrinsic" and "Dispose" in arg:
                f.add("syntax:using")
            elif op == "init_global_lex" or op == "global_decls":
                pass
            elif op == "put_global":
                name = atom(arg)
                if name not in BUILTIN_GLOBALS and name not in CODEMODE_GLOBALS:
                    f.add("sloppy:implicit-global-assignment")
                    declared_globals.add(name)
            elif op in ("get_global", "get_global_typeof"):
                name = atom(arg)
                if name in declared_globals:
                    continue
                if name in BUILTIN_GLOBALS:
                    g = "TypedArray" if name in TYPED_ARRAYS else name
                    f.add(f"global:{g}")
                    if nxt[0] in ("get_field", "get_method") and op == "get_global":
                        member = atom(nxt[1])
                        if member is not None:
                            f.add(f"static:{g}.{member}")
                    if name == "RegExp":
                        # `new RegExp("lit", "flags")`: the pattern, when it is a constant.
                        pat = const_string(nxt[1]) if nxt[0] == "push_const" else None
                        if pat is not None:
                            fl = ops[i + 2]
                            flags = const_string(fl[1]) if fl[0] == "push_const" else ""
                            f |= regex_features(pat, flags or "")
                        else:
                            f.add("regex:dynamic-pattern")
                elif name in CODEMODE_GLOBALS:
                    f.add(f"codemode:{name}")
                elif name in QJS_ONLY_GLOBALS:
                    f.add(f"qjs-only-global:{name}")
                else:
                    f.add(f"unknown-global:{name}")
            elif op == "get_method":
                name = atom(arg)
                if name is not None and prev[0] != "get_global":
                    f.add(f"call:.{name}")
            elif op in ("get_field", "get_field2"):
                name = atom(arg)
                # for-of, destructuring and yield* read next/done/value/return/throw
                # themselves: not the script's own iterator protocol.
                generated = name in ITER_FIELDS and name not in dotted
                if name is not None and prev[0] != "get_global" and not generated:
                    f.add(f"read:.{name}")
            elif op == "put_field":
                name = atom(arg)
                if name is not None:
                    f.add(f"write:.{name}")
    return f


def token_features(code):
    f = set()
    try:
        toks = [t for t in jstok.tokenize(code) if t[0] not in ("ws", "comment")]
    except jstok.TokenizeError:
        return {"token:untokenizable"}
    for i, (kind, text) in enumerate(toks):
        nxt = toks[i + 1] if i + 1 < len(toks) else ("", "")
        if kind == "word" and text in ("break", "continue") and nxt[0] == "word" and nxt[1] not in (
            "case", "default", "if", "else", "return", "for", "while", "const", "let", "var", "}"
        ):
            f.add("syntax:label")
        elif kind == "word" and text == "new" and nxt[1] == "." and i + 2 < len(toks) and toks[i + 2][1] == "target":
            f.add("syntax:new.target")
        elif kind == "word" and text == "with" and nxt[1] == "(" and (i == 0 or toks[i - 1][1] != "."):
            f.add("sloppy:with")
        elif kind == "number" and re.fullmatch(r"0[0-7]+", text):
            f.add("sloppy:legacy-octal-literal")
        elif kind == "string" and re.search(r"(?<!\\)(\\\\)*\\(0[0-9]|[1-7])", text):
            f.add("sloppy:octal-escape")
        elif kind == "word" and text == "using" and nxt[0] == "word":
            f.add("syntax:using")
        # The everyday syntax, for contrast (the bytecode does not tell these apart).
        if kind == "word" and text in CORE_WORDS and (i == 0 or toks[i - 1][1] not in (".", "?.")):
            f.add(f"core:{text}")
        elif kind == "punct" and text in CORE_PUNCT:
            f.add(f"core:{CORE_PUNCT[text]}")
        elif kind == "template":
            f.add("core:template-literal")
        if kind == "word" and text in ("const", "let", "var") and nxt[1] in ("{", "["):
            f.add("core:destructuring-declaration")
    return f


def dotted_names(code):
    try:
        toks = [t for t in jstok.tokenize(code) if t[0] not in ("ws", "comment")]
    except jstok.TokenizeError:
        return frozenset(ITER_FIELDS)
    return frozenset(b[1] for a, b in zip(toks, toks[1:]) if a[1] in (".", "?.") and b[0] == "word")


def analyze(code, djs):
    """(features, compile error) of one script."""
    with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as t:
        t.write(code)
        path = t.name
    try:
        r = subprocess.run([djs, "disasm", "--body", path], capture_output=True, text=True, timeout=60)
    finally:
        os.unlink(path)
    feats = token_features(code)
    if r.returncode != 0:
        err = r.stderr.strip().split(": ", 1)[-1] if r.stderr else "failed"
        # Positions differ per script; keep the message only.
        err = re.sub(r"^\S+:\d+:\d+: ", "", r.stderr.strip())
        return sorted(feats), err
    return sorted(feats | bytecode_features(parse_disasm(r.stdout), dotted_names(code))), None


def scan(extract, out, djs, jobs):
    runs = [json.loads(line) for line in open(extract)]
    codes = {}
    for r in runs:
        # The options line is a comment to the engine (pi replaces it with an empty line).
        codes.setdefault(r["code"], None)
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        futs = {ex.submit(analyze, c, djs): c for c in codes}
        for fut in concurrent.futures.as_completed(futs):
            codes[futs[fut]] = fut.result()
    with open(out, "w") as f:
        for r in runs:
            feats, err = codes[r["code"]]
            f.write(json.dumps({
                "source": r["source"],
                "session": r["source"].split(":")[0],
                "timestamp": r.get("timestamp"),
                "script": hashlib.sha256(r["code"].encode()).hexdigest()[:12],
                "isError": r.get("isError"),
                "features": feats,
                "compileError": err,
            }) + "\n")
    errs = sum(1 for v in codes.values() if v[1])
    print(f"features: {len(runs)} executions, {len(codes)} distinct scripts, {errs} do not compile in djs", file=sys.stderr)


def summary(out, prelude_feats):
    rows = [json.loads(line) for line in open(out)]
    total = len(rows)
    scripts = len({r["script"] for r in rows})
    sessions = len({r["session"] for r in rows})
    per = collections.defaultdict(lambda: [0, set(), set(), None, None])
    for r in rows:
        feats = list(r["features"])
        if r["compileError"]:
            feats.append("compile-error:" + r["compileError"])
        for k in feats:
            p = per[k]
            p[0] += 1
            p[1].add(r["script"])
            p[2].add(r["session"])
            d = (r["timestamp"] or "")[:10]
            if d:
                p[3] = min(p[3] or d, d)
                p[4] = max(p[4] or d, d)
    print(f"{total} executions, {scripts} distinct scripts, {sessions} sessions\n")
    print("| feature | executions | scripts | sessions | first | last | prelude |")
    print("| --- | ---: | ---: | ---: | --- | --- | --- |")
    for k, (n, s, ss, a, b) in sorted(per.items(), key=lambda kv: (kv[0].split(":")[0], -kv[1][0], kv[0])):
        print(f"| {k} | {n} | {len(s)} | {len(ss)} | {a} | {b} | {'yes' if k in prelude_feats else ''} |")


def main():
    args = sys.argv[1:]
    djs = "target/debug/djs"
    jobs = 6
    prelude = None
    if "--djs" in args:
        i = args.index("--djs")
        djs = args[i + 1]
        del args[i:i + 2]
    if "-j" in args:
        i = args.index("-j")
        jobs = int(args[i + 1])
        del args[i:i + 2]
    if "--prelude" in args:
        i = args.index("--prelude")
        prelude = args[i + 1]
        del args[i:i + 2]
    if args and args[0] == "--summary":
        pf = set()
        if prelude:
            # pi's prelude is a Script (a function expression), not a codemode body.
            r = subprocess.run([djs, "disasm", prelude], capture_output=True, text=True, check=True)
            src = open(prelude).read()
            pf = bytecode_features(parse_disasm(r.stdout), dotted_names(src)) | token_features(src)
        summary(args[1], pf)
        return
    scan(args[0], args[1], djs, jobs)


if __name__ == "__main__":
    main()
