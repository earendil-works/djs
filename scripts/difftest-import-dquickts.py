#!/usr/bin/env python3
"""Split dquickts' test files into difftest corpus cases, one per test function.

    scripts/difftest-import-dquickts.py [--dquickts DIR]

Reads tests/test_{language,builtin,closure,loop,decorators}.js and tests/test_typescript.ts from
dquickts (read-only, default ~/Development/dquickts) and writes
tests/corpus/dquickts/<file>/<test>.{js,ts}. Each case holds the helpers the test needs
(found by name, transitively), the test function and its call, as the body of a codemode
script. Top-level statements that are not declarations or test calls become a
`toplevel` case. Previously generated cases (and their .expected files) that are no
longer produced are deleted. Afterwards record the expectations with
`djs-difftest --update dquickts`.

Tests that cannot work in the codemode model are left out (EXCLUDE, with the reason).
Cases that read the caller's locals through eval (direct eval, a djs non-goal, D014) get
`// difftest: skip-djs` (DIRECT_EVAL, kept by hand: indirect eval and the Function
constructors work since 0059).
Cases in NO_STRESS get `// difftest: no-stress` (too slow for the stress modes, D1530).
Only the Python standard library is used.
"""

import argparse
import json
import os
import re
import sys

SOURCES = [
    ("test_language.js", "language"),
    ("test_builtin.js", "builtin"),
    ("test_closure.js", "closure"),
    ("test_loop.js", "loop"),
    ("test_decorators.js", "decorators"),
    ("test_typescript.ts", "typescript"),
]

# (file, test) -> why the stress modes skip it (`difftest: no-stress`, D1530).
NO_STRESS = {
    ("test_builtin.js", "test_map"): "over 120 s suspended, 65 s under GC stress (debug, 0053)",
    ("test_builtin.js", "test_rope"): "200k rope operations: over 120 s in each stress mode (0053)",
}

# (file, test) -> why it is not imported. Found by running every candidate with qjs in
# the codemode model (strict async function body): these fail there by design.
SLOPPY = "relies on sloppy mode; codemode scripts are strict"
EXCLUDE = {
    ("test_builtin.js", "test_finalization_registry"): "uses qjs's std.gc and os.setTimeout",
    ("test_builtin.js", "test_queue_microtask"): "uses qjs's os.setTimeout",
    ("test_builtin.js", "test_weak_map"): "uses qjs's std.gc",
    ("test_builtin.js", "test_weak_map_cycles"): "uses qjs's std.gc",
    ("test_builtin.js", "test_weak_ref"): "uses qjs's std.gc",
    ("test_builtin.js", "test_line_column_numbers"): "checks stack trace positions of eval'd code",
    ("test_closure.js", "test_with"): SLOPPY + " (`with`)",
    ("test_language.js", "test_labels"): SLOPPY + " (`with`)",
    ("test_language.js", "test_argument_scope"): SLOPPY + " (`eval` as a parameter name)",
    ("test_language.js", "test_global_var_opt"): SLOPPY + " (`delete` of a variable)",
    ("test_language.js", "test_number_literals"): SLOPPY + " (legacy octal literals)",
    ("test_language.js", "test_function_expr_name"): SLOPPY + " (assignment to a function expression's name)",
    ("test_language.js", "test_object_literal"): SLOPPY + " (assignment to an undeclared variable)",
    ("test_loop.js", "test_for_in"): SLOPPY + " (for-in initializer)",
    ("test_typescript.ts", "test_js_compatibility"): "compares two evalTS results; see corpus/ts/js-compat-as-*",
    ("test_typescript.ts", "test_modules"): "loads modules; codemode has none",
    ("test_typescript.ts", "test_using_syntax_1"): "`using x: T = null` is valid in a function body (an error only at script top level)",
}

# Cases that need direct eval: their eval'd source reads or writes the caller's locals.
DIRECT_EVAL = {
    ("test_builtin.js", "test_eval"),
    ("test_closure.js", "test_arrow_function"),
    ("test_closure.js", "test_eval_closure"),
    ("test_closure.js", "test_eval_const"),
    ("test_decorators.js", "test_misc"),
}

IDENT = re.compile(r"[A-Za-z_$][\w$]*")


def scan(src):
    """Splits a source file into top-level chunks (lists of whole lines).

    A chunk starts at a line that begins in column 0 while the scanner is at nesting
    depth 0 and outside strings, templates and comments, unless that line starts with
    a closing bracket, an opening brace (K&R function bodies) or a dot. A small lexer
    tracks strings, template literals (with ${} nesting), comments and regex literals
    (by the usual previous-token heuristic)."""
    lines = src.split("\n")
    chunks = []
    current = []
    depth = 0
    mode = None  # None, "'", '"', "`", "/*"
    template_depth = []  # brace depth at each open ${
    prev = ""  # previous significant character, for the regex heuristic
    for line in lines:
        at_top = depth == 0 and mode is None
        starts_chunk = (
            at_top
            and line[:1] not in ("", " ", "\t", "}", ")", "]", "{", ".")
        )
        if starts_chunk and current:
            chunks.append(current)
            current = []
        current.append(line)
        i = 0
        n = len(line)
        while i < n:
            c = line[i]
            if mode == "/*":
                if line.startswith("*/", i):
                    mode = None
                    i += 2
                    continue
                i += 1
                continue
            if mode in ("'", '"'):
                if c == "\\":
                    i += 2
                    continue
                if c == mode:
                    mode = None
                    prev = "a"
                i += 1
                continue
            if mode == "`":
                if c == "\\":
                    i += 2
                    continue
                if c == "`":
                    mode = None
                    prev = "a"
                elif line.startswith("${", i):
                    template_depth.append(depth)
                    depth += 1
                    mode = None
                    i += 2
                    continue
                i += 1
                continue
            # code
            if line.startswith("//", i):
                break
            if line.startswith("/*", i):
                mode = "/*"
                i += 2
                continue
            if c in "'\"`":
                mode = c
                i += 1
                continue
            if c == "/":
                if prev in ("", "(", ",", "=", ":", "[", "!", "&", "|", "?", "{", "}", ";", "+", "-", "*", "%", "<", ">", "~", "^") or prev == "kw":
                    # regex literal
                    i += 1
                    in_class = False
                    while i < n:
                        d = line[i]
                        if d == "\\":
                            i += 2
                            continue
                        if d == "[":
                            in_class = True
                        elif d == "]":
                            in_class = False
                        elif d == "/" and not in_class:
                            i += 1
                            break
                        i += 1
                    while i < n and (line[i].isalnum()):
                        i += 1
                    prev = "a"
                    continue
                prev = "/"
                i += 1
                continue
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
                if c == "}" and template_depth and template_depth[-1] == depth:
                    template_depth.pop()
                    mode = "`"
                    i += 1
                    continue
            if c.isalnum() or c in "_$":
                m = IDENT.match(line, i)
                if m:
                    word = m.group(0)
                    prev = "kw" if word in ("return", "typeof", "case", "do", "else", "in", "of", "void", "yield", "await") else "a"
                    i = m.end()
                    continue
            if not c.isspace():
                prev = c
            i += 1
        # a line comment ends at the end of the line; strings do not continue (no
        # line continuations in these files)
        if mode in ("'", '"'):
            mode = None
    if current:
        chunks.append(current)
    return chunks


def strip_code(text):
    """Text with comments and string contents blanked, for finding identifiers."""
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    text = re.sub(r"//[^\n]*", " ", text)
    text = re.sub(r"'(?:\\.|[^'\\\n])*'", "''", text)
    text = re.sub(r'"(?:\\.|[^"\\\n])*"', '""', text)
    return text


class Chunk:
    def __init__(self, lines):
        self.text = "\n".join(lines).rstrip() + "\n"
        code = strip_code(self.text).strip()
        self.kind = "other"
        self.name = None
        self.defines = set()
        m = re.match(r"(?:export\s+)?(async\s+)?function\s*\*?\s*([A-Za-z_$][\w$]*)", code)
        if m:
            self.kind = "function"
            self.name = m.group(2)
            self.is_async = bool(m.group(1))
            self.defines = {self.name}
            return
        m = re.match(r"(await\s+)?([A-Za-z_$][\w$]*)\(\);?$", code)
        if m:
            self.kind = "call"
            self.name = m.group(2)
            self.awaited = bool(m.group(1))
            return
        if code == "" or code.startswith("//"):
            self.kind = "comment"
            return
        raw = re.sub(r"//[^\n]*", "", self.text).strip()
        if re.match(r"['\"]use strict['\"];?$", raw):
            self.kind = "drop"
            return
        if "__loadScript" in code or re.match(r"import\s+\*\s+as\s+std\b", code):
            self.kind = "drop"
            return
        # (string contents are blanked in `code`, so match the import on the raw text)
        if re.match(r"import\s*\{[^}]*\}\s*from\s*['\"]\./assert\.js['\"];?$", raw):
            self.kind = "import-assert"
            return
        m = re.match(r"(?:var|let|const|class)\s+([A-Za-z_$][\w$]*)", code)
        if m:
            self.kind = "declaration"
            self.defines = {m.group(1)}
            for extra in re.findall(r",\s*([A-Za-z_$][\w$]*)\s*=", code.split("\n")[0]):
                self.defines.add(extra)
            return
        self.kind = "statement"

    def references(self):
        return set(IDENT.findall(strip_code(self.text)))


def load_chunks(src):
    chunks = [Chunk(lines) for lines in scan(src)]
    # Attach comment-only chunks to the chunk that follows.
    merged = []
    pending = ""
    for c in chunks:
        if c.kind == "comment":
            pending += c.text
            continue
        if pending:
            c.text = pending + c.text
            pending = ""
        merged.append(c)
    return merged


def assert_module_chunks(dquickts):
    with open(os.path.join(dquickts, "tests", "assert.js"), encoding="utf-8") as f:
        src = f.read().replace("export function", "function")
    return [c for c in load_chunks(src) if c.kind == "function"]


def build_cases(path, stem, dquickts):
    with open(path, encoding="utf-8") as f:
        src = f.read()
    chunks = load_chunks(src)
    helpers = []  # chunks that define names (in file order)
    for c in chunks:
        if c.kind == "import-assert":
            helpers.extend(assert_module_chunks(dquickts))
        elif c.kind in ("function", "declaration"):
            helpers.append(c)
    functions = {c.name: c for c in chunks if c.kind == "function"}
    calls = [c for c in chunks if c.kind == "call" and c.name in functions]
    tests = {c.name for c in calls}
    statements = [c for c in chunks if c.kind == "statement" or (c.kind == "call" and c.name not in functions)]
    defined = {}
    for h in helpers:
        for name in h.defines:
            defined.setdefault(name, h)

    def closure(roots):
        needed = []
        seen = set()
        work = list(roots)
        while work:
            c = work.pop()
            for name in c.references():
                h = defined.get(name)
                if h is None or id(h) in seen or h.name in tests:
                    continue
                seen.add(id(h))
                needed.append(h)
                work.append(h)
        order = {id(h): i for i, h in enumerate(helpers)}
        return sorted(needed, key=lambda h: order[id(h)])

    cases = []
    for call in calls:
        fn = functions[call.name]
        note = None
        if stem == "typescript":
            fn, note, syntax_cases = split_eval_checks(fn)
            for i, (src, ts) in enumerate(syntax_cases, 1):
                cases.append((f"{call.name}_syntax_{i}", [src], ".ts" if ts else ".js",
                              f"assertSyntaxError #{i} of {call.name}: must be a SyntaxError"))
        body = [h.text for h in closure([fn])] + [fn.text, call.text]
        cases.append((call.name, body, None, note))
    if statements:
        body = [h.text for h in closure(statements)] + [s.text for s in statements]
        cases.append(("toplevel", body, None, None))
    return cases


SYNTAX_CHECK = re.compile(r'^\s*assertSyntaxError\(("(?:\\.|[^"\\])*")(\s*,\s*false)?\);\s*$')


def split_eval_checks(fn):
    """test_typescript.ts checks syntax errors with evalTS (qjs's std.evalScript), which
    a codemode script cannot do. Each single-line `assertSyntaxError("src"[, false])`
    becomes its own case whose source is `src` (TypeScript, or JavaScript with
    `false`); other lines that call evalTS are dropped. Returns the rewritten function
    chunk, a note for the header, and the (src, is_ts) syntax cases."""
    kept = []
    syntax = []
    dropped = 0
    for line in fn.text.split("\n"):
        m = SYNTAX_CHECK.match(line)
        if m:
            syntax.append((json.loads(m.group(1)), m.group(2) is None))
        elif "evalTS(" in line:
            dropped += 1
        else:
            kept.append(line)
    if not syntax and not dropped:
        return fn, None, []
    new = Chunk(kept)
    new.text = "\n".join(kept)
    parts = []
    if syntax:
        parts.append(f"{len(syntax)} assertSyntaxError checks moved to {fn.name}_syntax_N cases")
    if dropped:
        parts.append(f"{dropped} other evalTS lines dropped")
    return new, "; ".join(parts), syntax


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--dquickts", default=os.path.expanduser("~/Development/dquickts"))
    args = parser.parse_args()
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
    out_root = os.path.join(root, "tests", "corpus", "dquickts")
    written = set()
    excluded = []
    for filename, stem in SOURCES:
        path = os.path.join(args.dquickts, "tests", filename)
        ext = os.path.splitext(filename)[1]
        out_dir = os.path.join(out_root, stem)
        os.makedirs(out_dir, exist_ok=True)
        for name, body, case_ext, note in build_cases(path, stem, args.dquickts):
            reason = EXCLUDE.get((filename, name))
            if reason:
                excluded.append(f"{filename} {name}: {reason}")
                continue
            text = "\n".join(part.rstrip("\n") + "\n" for part in body)
            header = [
                f"// Imported from dquickts tests/{filename} ({name}) by",
                "// scripts/difftest-import-dquickts.py; edit the script, not this file.",
            ]
            if note:
                header.append(f"// {note}.")
            if (filename, name) in DIRECT_EVAL:
                header.append(
                    "// difftest: skip-djs direct eval: access to the caller's scope is a non-goal (D014)"
                )
            if (filename, name) in NO_STRESS:
                header.append(f"// difftest: no-stress {NO_STRESS[(filename, name)]}")
            case_path = os.path.join(out_dir, name + (case_ext or ext))
            with open(case_path, "w", encoding="utf-8") as f:
                f.write("\n".join(header) + "\n\n" + text)
            written.add(os.path.normpath(case_path))
    # The tests are QuickJS's (MIT): keep its notice next to the copies.
    with open(os.path.join(args.dquickts, "LICENSE"), encoding="utf-8") as f:
        notice = f.read()
    with open(os.path.join(out_root, "LICENSE"), "w", encoding="utf-8") as f:
        f.write(notice)
    # Remove cases (and expectations) that are no longer generated.
    removed = 0
    for dirpath, _, files in os.walk(out_root):
        for f in files:
            p = os.path.normpath(os.path.join(dirpath, f))
            base, ext = os.path.splitext(p)
            if ext in (".js", ".ts") and p not in written:
                os.remove(p)
                removed += 1
            elif ext == ".expected" and base + ".js" not in written and base + ".ts" not in written:
                os.remove(p)
                removed += 1
    print(f"wrote {len(written)} cases, removed {removed} stale files, excluded {len(excluded)}:")
    for e in excluded:
        print("  " + e)


if __name__ == "__main__":
    sys.exit(main())
