#!/usr/bin/env python3
"""Code size per feature area, raw and gzip, with one taxonomy for djs and QuickJS, so the
two can be compared area by area (0123, docs/size-vs-quickjs.md).

djs:     scripts/size/areas.py djs SHIPPED.wasm NAMES.wasm [--list AREA] [--other]
QuickJS: scripts/size/areas.py c NAMED.wasm LINES.txt [--list AREA] [--other]

djs: the shipped module (`scripts/build-wasm.sh`) has no names. NAMES.wasm is a build of
the same code with a name section (`CARGO_PROFILE_WASM_STRIP=debuginfo`, or `--keep-names`,
whose relocations are padded but whose functions are the same, in the same order): the
names come from it, the sizes from SHIPPED.

QuickJS: a wasm build with a name section and DWARF line tables (`zig cc -g`, see
docs/size-vs-quickjs.md), LINES.txt from `llvm-dwarfdump --debug-line`. Each function
also gets the source file most of its bytes come from (headers excluded), so library
files (libregexp.c, libunicode.c, dtoa.c, libc) are recognized by file.

gzip per area: the area's function bodies, in module order, compressed on their own, then
scaled so that the areas add up to the gzip of the whole code section (an estimate: gzip
also matches across areas).
"""
import gzip
import importlib.util
import os
import re
import sys
from collections import Counter, defaultdict

here = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(here, "..", file))
    mod = importlib.util.module_from_spec(spec)
    src = open(spec.origin).read().replace("\nmain()\n", "\n")
    exec(compile(src, spec.origin, "exec"), mod.__dict__)
    return mod


wf = load("wasm_funcs", "wasm-funcs.py")
wl = load("wasm_dwarf_lines", "wasm-dwarf-lines.py")

COMPILER = "compiler (parser, scopes, codegen)"
LEXER = "lexer"
TS = "TypeScript"
LOOP = "interpreter: dispatch loop"
RUNTIME = "runtime core (calls, properties, objects, strings, atoms, operators, errors)"
GC = "GC + memory management"
REALM = "realm setup from tables"
BUILTINS = "builtins: object, array, string, function, iterators, promise, proxy, collections, typed arrays, math, ..."
NATIVES = "builtins: generated Rust (bgen natives)"
FAST = "builtins: hand-written fast paths, inline caches"
REGEXP = "regexp"
UNICODE = "unicode (case, properties, normalization)"
NUM = "number parse/format"
BIGINT = "BigInt"
JSON = "JSON"
DATE = "Date"
SNAP = "snapshots, suspension, wire format"
MODULES = "modules"
DEBUG = "debug dumps, memory statistics"
HOST = "host API / ABI"
STD = "std: fmt, panic, malloc, out-of-line generics / libc: printf, malloc, qsort"
LIBM = "libm, soft float"

DJS = [
    (TS, r"compiler::ts|Parser>::ts_|compiler::ts_decl|compiler::decorators|Parser>::(parse_)?decorator"),
    (REGEXP, r"djs_core::regexp::|builtins::regexp|json_regexp_intrinsic|regexp"),
    (JSON, r"djs_core::json::|builtins::json"),
    (BIGINT, r"djs_core::bigint::|builtins::bigint|interp::run::bigint|bigint"),
    (DATE, r"builtins::date"),
    (NUM, r"djs_core::num::"),
    (UNICODE, r"djs_core::unicode::|builtins::(uri|normalize)"),
    (LEXER, r"djs_core::lexer::"),
    (COMPILER, r"djs_core::compiler::|djs_core::bytecode::(code|template|imm|ops|stack)"),
    (NATIVES, r"builtins::gen::natives|interp::run::native"),
    (FAST, r"builtins::fast|interp::run::ic|builtins::dense"),
    (LOOP, r"interp::run::Exec>::(run|run_hot|step_once)$|interp::run::hot"),
    (BUILTINS, r"interp::run::Exec>::(call_intrinsic|fast_intrinsic)|interp::run::intrinsics|djs_core::builtins::|interp::run::(proxy|promise|collection|typed|iter)"),
    (REALM, r"interp::realm::(install|init|new_realm)|Realm>::new"),
    (SNAP, r"djs_core::(wire|image|snapshot)::|interp::run::suspend|Heap>::(from_bytes|to_bytes|verify|compact_for_image)|heap::image"),
    (GC, r"heap::(gc|walk|compact|compactor|arena|weakref)::|Heap>::(collect|collect_canonical|collect_leaves\w*|mark_compact\w*|copy_piece|trace\w*|scan\w*|cell_invariants|grow\w*)$|Heap>::(mark_compact|collect_canonical)::"),
    (BUILTINS, r"heap::collection|Heap>::collection_"),
    (RUNTIME, r"djs_core::interp::|djs_core::bytecode::|djs_core::heap::|djs_core::value|djs_core::sort"),
    (HOST, r"djs_core::vm::|djs_wasm::|^vm_|^djs_|std::thread::local"),
    (LIBM, r"^(djs_)?libm::|compiler_builtins::math|compiler_builtins::float|__(mul|add|div|sub)tf3"),
    (STD, r"djs_core::|core::|alloc::|std::|^<|panic|compiler_builtins|^__|^mem(cpy|set|move|cmp)|dlmalloc"),
]

C = [
    (LIBM, r"compiler_rt|\bc\.math|math\.|/math/|\|(pow|fma|exp|expm1|log|log2|log10|log1p|sin|cos|tan|asin|acos|atan|atan2|sinh|cosh|tanh|asinh|acosh|atanh|cbrt|hypot|fmod|frexp\w*|ldexp\w*|scalbn\w*|floor|ceil|trunc|round|lrint|rint)$|__\w+tf3|__\w+tf2"),
    (STD, r"libc|wasi|musl|\|(printf_core|vfprintf|snprintf|vsnprintf|wcrtomb|pop_arg|qsort|rqsort|memcpy|memmove|memset|memcmp|strlen|strcmp|strchr|malloc|free|realloc|calloc|dlmalloc|\w*malloc\w*)$|\|c\.|\|mem\.|\|__js_(malloc|free|realloc)|\|js_def_|\|js_(malloc|free|realloc|mallocz|strdup|strndup)\w*$|cutils\.c"),
    (REGEXP, r"libregexp\.c|\|(lre_|re_)|js_regexp|RegExp|regexp"),
    (UNICODE, r"libunicode\.c|\|(cr_|unicode_)|normaliz|to_nfd|js_string_normalize|case_conv"),
    (NUM, r"dtoa\.c|js_dtoa|js_atod|js_atof|js_fcvt|js_ecvt|u64toa|i64toa|u32toa|mul_pow|output_digits|js_number_to|js_number_proto|toFixed|toPrecision"),
    (TS, r"\|ts_|_ts_|js_decorator|decorator|enum_"),
    (BIGINT, r"bigint|BigInt|mpb_|js_limb"),
    (JSON, r"json|JSON"),
    (DATE, r"[Dd]ate|tzoffset|secs_to_tm|timezone|get_time"),
    (MODULES, r"module|Module|import|export|js_link|js_evaluate_module|exotic_ns|_ns_"),
    (DEBUG, r"Dump|dump_|ComputeMemoryUsage|compute_value_size|print_atom|JS_DumpMemory|js_dump"),
    (SNAP, r"ReadObject|WriteObject|bc_|snap_|JS_Read|JS_Write|_read_|_write_|serialize"),
    (LEXER, r"\|(next_token|simple_next_token|json_next_token|parse_ident|ident_realloc|free_token|token_|js_parse_string|js_parse_template_part|js_parse_regexp|get_line_col|find_line_num)"),
    (COMPILER, r"js_parse|\|resolve_|\|emit_|optimize|skip_dead|code_match|new_label|update_label|push_scope|pop_scope|add_var|add_arg|add_scope|add_module|close_scopes|instantiate_hoisted|js_create_function|js_new_function_def|js_free_function_def|cpool_|ss_check|compute_stack_size|__JS_EvalInternal|JS_EvalFunction|define_var|get_lvalue|put_lvalue|put_short_code|push_short_int|find_jump_target|is_label|find_var|find_arg|get_closure_var|add_closure|var_object_test|mark_eval|has_with|update_var|convert_fast|live_code|skip_|set_object_name|lvalue|label|scope|closure_var|private_field|add_brand|add_private|js_is_live|rv_emit|emit|capture_scope|hoisted|block_scope|fd_|cpool|func_def|_fd"),
    (LOOP, r"\|js_call_internal$|JS_CallInternal"),
    (REALM, r"JS_AddIntrinsic|JS_NewContext|js_init_|JS_SetPropertyFunctionList|JS_InstantiateFunctionList|JS_AutoInit|js_instantiate_prototype|JS_NewCConstructor|js_new_shape_hint|JS_NewGlobalCConstructor|JS_SetConstructor\b|init_class_range|JS_NewRuntime|js_create_module_function"),
    (GC, r"gc_|_gc|mark_|\|free_|_free\b|free_gc|JS_RunGC|JS_FreeRuntime|JS_FreeContext|__JS_FreeValue|js_free_|finalizer|_finalizer|reset_weak|weakref|zombie"),
    (HOST, r"\|qjs_|^embed|JS_SetHost|host_|JS_Set(Interrupt|MemoryLimit|MaxStack|Opaque|Runtime)|JS_Get(Runtime|Opaque)"),
    (BUILTINS, r"\|js_(array|string|object|function|iterator|promise|async|generator|proxy|reflect|map|set|weak|finrec|typed|TA|array_buffer|dataview|atomics|math|symbol|global|error|boolean|number|regexp|disposable|uint8array|base64|hex|array_iterator|callsite|bound|for_in|global_|structured)|Atomics|perform_promise|promise|fulfill|reject|array_from|map_|set_|iterator|b64|base64|from_base64|u8a_|hex_|js_error|js_symbol"),
    (RUNTIME, r"."),
]


def classify(rules, name):
    for area, pat in rules:
        if re.search(pat, name):
            return area
    return "other"


def bodies(path):
    b = open(path, "rb").read()
    i = 8
    while i < len(b):
        sid = b[i]
        size, start = wf.leb(b, i + 1)
        if sid == 10:
            n, k = wf.leb(b, start)
            out = []
            for _ in range(n):
                s, j = wf.leb(b, k)
                out.append(b[k:j + s])
                k = j + s
            return out, b[start:start + size]
        i = start + size
    raise SystemExit("no code section")


def c_names(named, lines):
    """`file|name` per function: the source file most of its bytes come from (headers
    and inlined library files count only if nothing else does)."""
    names = [n for n, _ in wf.functions(named)]
    ranges = wl.function_ranges(named)
    starts = sorted((s, e, i) for i, (s, e) in enumerate(ranges))
    per = [Counter() for _ in names]
    import bisect
    keys = [s for s, _, _ in starts]
    for seq in wl.parse_lines(lines):
        for (a, f, _), (b, _, _) in zip(seq, seq[1:]):
            if b <= a:
                continue
            k = bisect.bisect_right(keys, a) - 1
            if k >= 0 and a < starts[k][1]:
                per[starts[k][2]][f] += b - a
    out = []
    for n, c in zip(names, per):
        files = [f for f, _ in c.most_common() if not f.endswith(".h")]
        out.append(f"{files[0] if files else '?'}|{n}")
    return out


def main():
    args = sys.argv[1:]
    mode = args.pop(0)
    listing, show_other = None, False
    if "--list" in args:
        k = args.index("--list")
        listing = args[k + 1]
        del args[k:k + 2]
    if "--other" in args:
        args.remove("--other")
        show_other = True
    if mode == "djs":
        shipped, named = args
        names = [n for n, _ in wf.functions(named)]
        bods, code = bodies(shipped)
        rules = DJS
    else:
        named, lines = args
        names = c_names(named, lines)
        bods, code = bodies(named)
        rules = C
    if len(names) != len(bods):
        raise SystemExit(f"function counts differ: {len(names)} vs {len(bods)}")
    groups = defaultdict(list)
    for n, body in zip(names, bods):
        groups[classify(rules, n)].append((n, body))
    if listing or show_other:
        key = listing or "other"
        for area in groups:
            if area.startswith(key):
                for n, body in sorted(groups[area], key=lambda x: -len(x[1])):
                    print(f"{len(body):>8}  {n[:150]}")
        return
    total = len(code)
    gz_total = len(gzip.compress(code, 9))
    gz = {k: len(gzip.compress(b"".join(body for _, body in v), 9)) for k, v in groups.items()}
    scale = gz_total / sum(gz.values())
    print(f"{args[0]}: code {total:,} bytes, gzip {gz_total:,} ({len(bods)} functions)\n")
    print(f"{'raw':>9} {'%':>5} {'gzip':>8} {'funcs':>5}  area")
    for k, v in sorted(groups.items(), key=lambda kv: -sum(len(b) for _, b in kv[1])):
        raw = sum(len(b) for _, b in v)
        print(f"{raw:>9,} {100 * raw / total:5.1f} {round(gz[k] * scale):>8,} {len(v):>5}  {k}")


main()
