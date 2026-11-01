"""Synthetic structured tool results for the codemode harvest (0121, D2283).

A tool's result type comes from its declaration in the session (`name(args: ...):
Promise<T>`, the TypeScript that pi's codemode renders from the tool's output schema).
`parse` reads the subset of TypeScript those declarations use, and `value` builds a JSON
value of that type from synthetic words and numbers. Property keys and literal strings
go through the case's anonymizer, so they match the redacted script's property accesses.
Nothing of the session's data is used: only the shape.
"""

import re

WORDS = (
    "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda omega red green blue "
    "north south east west build test check value item entry record status ready done"
).split()

_TOKEN = re.compile(
    r"\s+|//[^\n]*|/\*.*?\*/|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|-?\d+(?:\.\d+)?"
    r"|[A-Za-z_$][\w$]*|\[\]|.",
    re.S,
)


class ParseError(Exception):
    pass


def tokens(text):
    out = []
    for m in _TOKEN.finditer(text):
        t = m.group()
        if t.isspace() or t.startswith("//") or t.startswith("/*"):
            continue
        out.append(t)
    return out


class Parser:
    def __init__(self, text):
        self.toks = tokens(text)
        self.i = 0

    def peek(self, k=0):
        return self.toks[self.i + k] if self.i + k < len(self.toks) else None

    def eat(self, t=None):
        tok = self.peek()
        if tok is None or (t is not None and tok != t):
            raise ParseError(f"expected {t!r}, got {tok!r}")
        self.i += 1
        return tok

    def type(self):
        if self.peek() == "|":
            self.eat()
        parts = [self.intersection()]
        while self.peek() == "|":
            self.eat()
            parts.append(self.intersection())
        return parts[0] if len(parts) == 1 else ("union", parts)

    def intersection(self):
        parts = [self.postfix()]
        while self.peek() == "&":
            self.eat()
            parts.append(self.postfix())
        return parts[0] if len(parts) == 1 else ("inter", parts)

    def postfix(self):
        t = self.primary()
        while self.peek() == "[]":
            self.eat()
            t = ("array", t)
        return t

    def generic_args(self):
        args = []
        self.eat("<")
        while self.peek() != ">":
            args.append(self.type())
            if self.peek() == ",":
                self.eat()
        self.eat(">")
        return args

    def primary(self):
        t = self.peek()
        if t is None:
            raise ParseError("end of type")
        if t == "{":
            return self.object()
        if t == "(":
            self.eat()
            inner = self.type()
            self.eat(")")
            return inner
        if t == "[":
            self.eat()
            items = []
            while self.peek() != "]":
                items.append(self.type())
                if self.peek() == ",":
                    self.eat()
            self.eat("]")
            return ("tuple", items)
        if t[0] in "\"'":
            self.eat()
            return ("lit", t[1:-1])
        if re.fullmatch(r"-?\d+(?:\.\d+)?", t):
            self.eat()
            return ("lit", float(t) if "." in t else int(t))
        self.eat()
        if t in ("true", "false"):
            return ("lit", t == "true")
        if self.peek() == "<":
            args = self.generic_args()
            if t == "Array" and args:
                return ("array", args[0])
            if t == "Record" and len(args) == 2:
                return ("record", args[1])
            return ("generic", t, args)
        return ("name", t)

    def object(self):
        self.eat("{")
        members = []
        index = None
        while self.peek() != "}":
            if self.peek() == "[":
                # `[key: string]: T`
                self.eat("[")
                while self.peek() != "]":
                    self.eat()
                self.eat("]")
                self.eat(":")
                index = self.type()
            else:
                key = self.eat()
                if key[0] in "\"'":
                    key = key[1:-1]
                optional = False
                if self.peek() == "?":
                    self.eat()
                    optional = True
                self.eat(":")
                members.append((key, optional, self.type()))
            if self.peek() in (";", ","):
                self.eat()
        self.eat("}")
        return ("object", members, index)


def parse(text):
    p = Parser(text)
    t = p.type()
    if p.peek() is not None:
        raise ParseError(f"trailing {p.peek()!r}")
    return t


def _words(rng, lo, hi):
    return " ".join(rng.choice(WORDS) for _ in range(rng.randint(lo, hi)))


def value(t, rng, anon, depth=0):
    """A JSON value of type `t`. `anon` renames keys and string literals like the case's
    script."""
    kind = t[0]
    if kind == "lit":
        return anon.identifier(t[1]) if isinstance(t[1], str) else t[1]
    if kind == "name":
        n = t[1]
        if n == "string":
            return _words(rng, 1, 4)
        if n == "number":
            return rng.randint(0, 500)
        if n == "boolean":
            return rng.random() < 0.6
        if n in ("null", "undefined", "void", "never"):
            return None
        if n in ("ContentBlock", "TextContent"):
            return {"type": "text", "text": _words(rng, 3, 10)}
        # unknown, any, object and type names the declarations do not define.
        return {"value": _words(rng, 1, 3)}
    if kind == "object":
        _, members, index = t
        out = {}
        for key, _optional, mt in members:
            if depth > 6:
                break
            out[anon.identifier(key)] = value(mt, rng, anon, depth + 1)
        if index is not None and not members and depth <= 6:
            for _ in range(2):
                out[anon.identifier(rng.choice(WORDS))] = value(index, rng, anon, depth + 1)
        return out
    if kind == "record":
        return {anon.identifier(rng.choice(WORDS)): value(t[1], rng, anon, depth + 1) for _ in range(2)}
    if kind == "array":
        n = 0 if depth > 5 else rng.randint(1, 3)
        return [value(t[1], rng, anon, depth + 1) for _ in range(n)]
    if kind == "tuple":
        return [value(x, rng, anon, depth + 1) for x in t[1]]
    if kind == "union":
        variants = [v for v in t[1] if v not in (("name", "null"), ("name", "undefined"))]
        return value(rng.choice(variants or t[1]), rng, anon, depth)
    if kind == "inter":
        out = {}
        for part in t[1]:
            v = value(part, rng, anon, depth)
            if isinstance(v, dict):
                out.update(v)
        return out
    if kind == "generic":
        _, name, args = t
        if name == "CallToolResult":
            structured = value(args[0], rng, anon, depth + 1) if args else None
            text = _words(rng, 4, 12) if structured is None else None
            result = {"content": [{"type": "text", "text": text if text is not None else "(structured)"}]}
            if structured is not None:
                result["structuredContent"] = structured
            return result
        if name == "CloudflareResponse":
            return {
                "success": True,
                "status": 200,
                "result": value(args[0], rng, anon, depth + 1) if args else None,
                "errors": [],
                "messages": [],
            }
        if name == "Promise" and args:
            return value(args[0], rng, anon, depth)
        return {"value": _words(rng, 1, 3)}
    raise ParseError(f"unknown type node {kind}")


def like(v, rng, anon, depth=0):
    """A synthetic value of the same shape as a stored value: keys renamed like the
    script's, strings replaced by words of a similar length, numbers by numbers of the
    same magnitude."""
    if isinstance(v, dict):
        return {anon.identifier(k): like(x, rng, anon, depth + 1) for k, x in list(v.items())[:40]}
    if isinstance(v, list):
        return [like(x, rng, anon, depth + 1) for x in v[:12]]
    if isinstance(v, bool) or v is None:
        return v
    if isinstance(v, int):
        digits = len(str(abs(v)))
        n = rng.randint(10 ** (digits - 1) if digits > 1 else 0, 10**digits - 1)
        return -n if v < 0 else n
    if isinstance(v, float):
        return round(rng.uniform(0, abs(v) * 2 + 1), 3)
    if isinstance(v, str):
        target = min(len(v), 200)
        out = []
        while sum(len(w) + 1 for w in out) < target:
            out.append(rng.choice(WORDS))
        return " ".join(out)[:target]
    return None
