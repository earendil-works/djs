#!/usr/bin/env python3
"""Steps 2-4 of the codemode corpus harvest (0118, D2280): select, redact, emit.

Reads the executions extract.py wrote (private, work/scratch/0118/), keeps the scripts
whose tools have plain string results (read, bash, ls, grep, find) or that call no tool,
deduplicates them, picks a representative set by the language features they use,
redacts them, and writes one difftest case per script into OUTDIR, plus
OUTDIR/REDACTION.md: what was redacted in each case (kinds and counts, never values) and
what needs a human look. Tool results are not in the sessions; every case gets synthetic
results of the recorded calls' kinds.

    scripts/harvest/build.py EXTRACT OUTDIR [--count N] [--max-len CHARS]

Nothing from the sessions is printed. Review every case before committing it.
"""

import argparse
import collections
import hashlib
import json
import math
import os
import random
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
from anon import Anonymizer  # noqa: E402
from jstok import TokenizeError, tokenize  # noqa: E402

STRING_TOOLS = {"read", "bash", "ls", "grep", "find"}
# Scripts that use these need hosts or APIs the corpus does not fake (`load` needs values
# stored by earlier runs, which the sessions do not have).
EXCLUDED_GLOBALS = re.compile(
    r"\b(?:models|image|searchTools|describeTool|describeNamespace|fetch|require|load)\s*[.(]|^\s*(?:import|export)\s",
    re.M,
)

FEATURES = {
    "await": r"\bawait\b",
    "promise-all": r"Promise\.(?:all|allSettled|race|any)\b",
    "map": r"\.map\(",
    "filter": r"\.filter\(",
    "reduce": r"\.reduce\(",
    "forEach": r"\.forEach\(",
    "flatMap": r"\.flat(?:Map)?\(",
    "for-of": r"\bfor\s*\(\s*(?:const|let|var)\s+[^;]*\bof\b",
    "for-in": r"\bfor\s*\(\s*(?:const|let|var)\s+\w+\s+in\b",
    "for-classic": r"\bfor\s*\([^;)]*;[^;)]*;",
    "while": r"\bwhile\s*\(",
    "regex": r"(?:^|[=(,:!&|?{};]\s*)/[^/*\s][^/\n]*/[gimsuyd]*",
    "match": r"\.(?:match|matchAll|exec|test)\(",
    "replace": r"\.replace(?:All)?\(",
    "split": r"\.split\(",
    "join": r"\.join\(",
    "json-parse": r"JSON\.parse\(",
    "json-stringify": r"JSON\.stringify\(",
    "template": r"`",
    "destructuring": r"(?:const|let|var)\s*[\[{]",
    "spread": r"\.\.\.",
    "object-static": r"Object\.(?:entries|keys|values|fromEntries|assign)\(",
    "sort": r"\.sort\(",
    "set": r"\bnew Set\b",
    "map-coll": r"\bnew Map\b",
    "try": r"\btry\s*\{",
    "throw": r"\bthrow\b",
    "class": r"\bclass\s+\w",
    "function-decl": r"\bfunction\s+\w",
    "async-fn": r"\basync\s+(?:function|\(|\w+\s*=>)",
    "optional-chain": r"\?\.",
    "nullish": r"\?\?",
    "pad": r"\.pad(?:Start|End)\(",
    "slice": r"\.(?:slice|substring|substr)\(",
    "string-search": r"\.(?:includes|startsWith|endsWith|indexOf|lastIndexOf)\(",
    "number": r"\b(?:parseInt|parseFloat|Number)\(|\.toFixed\(",
    "math": r"\bMath\.\w+",
    "date": r"\bDate\b",
    "text": r"\btext\(",
    "console": r"\bconsole\.\w+\(",
    "store": r"\b(?:store|load)\(",
    "trim": r"\.trim(?:Start|End)?\(",
    "case": r"\.to(?:Upper|Lower)Case\(",
    "array-from": r"\bArray\.(?:from|isArray)\(",
    "length": r"\.length\b",
    "ternary": r"\?[^.?:]+:",
    "label-continue": r"\b(?:continue|break)\b",
    "exit": r"\bexit\(",
    "return": r"^\s*return\b",
    "generator": r"function\s*\*",
    "localeCompare": r"\.localeCompare\(",
}
FEATURE_RES = {k: re.compile(v, re.M) for k, v in FEATURES.items()}

# Path components and words kept as they are: generic layout names.
GENERIC_COMPONENTS = {
    "", ".", "..", "~", "src", "lib", "test", "tests", "docs", "doc", "packages", "apps", "crates",
    "node_modules", "dist", "build", "target", "scripts", "examples", "bin", "tmp", "var",
    "etc", "usr", "home", "user", "public", "assets", "config", "fixtures", "core", "utils",
    "index.ts", "index.js", "main.rs", "lib.rs", "mod.rs", "README.md", "package.json",
    "Cargo.toml", "tsconfig.json", ".git", "*", "**",
}
TLDS = "com|net|org|io|dev|ai|app|sh|co|cloud|internal|local|lan|me|xyz|so|gg|tech|run|page"
HOST_ALLOW = {"example.com", "example.org", "localhost"}
SECRET_RES = [
    ("key", re.compile(r"\b(?:sk|pk|rk)-[A-Za-z0-9_-]{16,}")),
    ("key", re.compile(r"\b(?:ghp|gho|ghu|ghs|ghr|github_pat)_[A-Za-z0-9_]{20,}")),
    ("key", re.compile(r"\bxox[abprs]-[A-Za-z0-9-]{10,}")),
    ("key", re.compile(r"\bAKIA[0-9A-Z]{16}\b")),
    ("key", re.compile(r"\bAIza[0-9A-Za-z_-]{30,}")),
    ("key", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    ("token", re.compile(r"(?i)\b(?:bearer|token|apikey|api_key|password|passwd|secret)\s*[:=]\s*[^\s,'\"`]{6,}")),
    ("token", re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")),
    ("connection-string", re.compile(r"\b(?:postgres|postgresql|mysql|mongodb|redis|amqp)(?:\+\w+)?://[^\s'\"`]+")),
]
EMAIL_RE = re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")
URL_RE = re.compile(r"\bhttps?://[^\s'\"`<>)\\]+")
IPV4_RE = re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b")
HOST_RE = re.compile(r"\b(?:[a-z0-9-]+\.)+(?:" + TLDS + r")\b", re.I)
HOME_RE = re.compile(r"(?:/Users|/home)/([^/\s'\"`$]+)")
# No backslashes: an escape sequence (`\"`) must survive renaming.
PATH_RE = re.compile(r"(?:~|\.{1,2})?(?:/[^\s'\"`$*:,;()\[\]{}<>|=\\]+)+/?|(?:[\w.@-]+/)+[\w.@-]+")
HIGH_ENTROPY_RE = re.compile(r"[A-Za-z0-9+_=-]{24,}")
NAME_RE = re.compile(r"\b[A-Z][a-z]{2,}\s+[A-Z][a-z]{2,}\b")


def entropy(s):
    counts = collections.Counter(s)
    return -sum(c / len(s) * math.log2(c / len(s)) for c in counts.values())


def synthetic_like(s, seed):
    """A string of the same length and character classes as `s`."""
    rng = random.Random(hashlib.sha256((seed + s).encode()).digest())
    out = []
    for ch in s:
        if ch.isdigit():
            out.append(rng.choice("0123456789"))
        elif ch.isupper():
            out.append(rng.choice("ABCDEFGHIJKLMNOPQRSTUVWXYZ"))
        elif ch.islower():
            out.append(rng.choice("abcdefghijklmnopqrstuvwxyz"))
        else:
            out.append(ch)
    return "".join(out)


class Redactor:
    """Redacts the text of literals in one case, with mappings that stay consistent within
    the case, and counts what it did by kind."""

    def __init__(self, case_id, usernames):
        self.case_id = case_id
        self.usernames = usernames
        self.counts = collections.Counter()
        self.flags = collections.Counter()
        self.components = {}
        self.hosts = {}
        self.emails = {}
        self.anon = Anonymizer(case_id)

    def component(self, c):
        if c in GENERIC_COMPONENTS or c.startswith("$") or not c:
            return c
        if c in self.components:
            return self.components[c]
        stem, dot, ext = c.rpartition(".")
        if dot and stem and len(ext) <= 5 and ext.isalnum():
            new = f"file{len(self.components) + 1}.{ext}"
        else:
            new = f"dir{len(self.components) + 1}"
        self.components[c] = new
        return new

    def path(self, m):
        p = m.group()
        parts = p.split("/")
        if len(parts) < 2 or (len(parts) == 2 and not parts[0] and "." not in parts[1] and len(parts[1]) < 4):
            return p
        self.counts["path"] += 1
        return "/".join(self.component(c) for c in parts)

    def host(self, h):
        h = h.lower()
        if h in HOST_ALLOW:
            return h
        if h not in self.hosts:
            self.hosts[h] = f"host{len(self.hosts) + 1}.example.com"
        self.counts["host"] += 1
        return self.hosts[h]

    def text(self, s):
        """Redacts literal text (string contents, template chunks, regex sources)."""
        for kind, rx in SECRET_RES:
            s, k = rx.subn(lambda m: "REDACTED_" + kind.upper().replace("-", "_"), s)
            self.counts[kind] += k

        def email(m):
            e = m.group().lower()
            if e not in self.emails:
                self.emails[e] = f"user{len(self.emails) + 1}@example.com"
            self.counts["email"] += 1
            return self.emails[e]

        s = EMAIL_RE.sub(email, s)

        def url(m):
            u = m.group()
            rest = u.split("://", 1)[1]
            host, _, path = rest.partition("/")
            host = host.split("@")[-1].split(":")[0]
            new_host = self.host(host) if host.lower() not in HOST_ALLOW else host
            self.counts["url"] += 1
            path = "/".join(self.component(c) for c in path.split("/")) if path else ""
            return "https://" + new_host + ("/" + path if path else "")

        s = URL_RE.sub(url, s)

        def ip(m):
            parts = m.group().split(".")
            if all(int(p) <= 255 for p in parts) and m.group() not in ("127.0.0.1", "0.0.0.0"):
                self.counts["ip"] += 1
                return "192.0.2." + str(int(parts[-1]) % 250 + 1)
            return m.group()

        s = IPV4_RE.sub(ip, s)
        s = HOST_RE.sub(lambda m: self.host(m.group()), s)

        def home(m):
            self.counts["home-dir"] += 1
            return "/home/user"

        s = HOME_RE.sub(home, s)
        s = PATH_RE.sub(self.path, s)
        for u in self.usernames:
            s, k = re.subn(r"\b" + re.escape(u) + r"\b", "user", s)
            self.counts["user-name"] += k

        def high_entropy(m):
            t = m.group()
            if entropy(t) >= 3.8 and re.search(r"\d", t) and re.search(r"[A-Za-z]", t):
                self.counts["high-entropy"] += 1
                return synthetic_like(t, self.case_id)
            return t

        s = HIGH_ENTROPY_RE.sub(high_entropy, s)
        for m in NAME_RE.finditer(s):
            self.flags["capitalized word pair (possible name)"] += 1
        return self.anon.text(s)


COMMAND_RE = re.compile(
    r"^[`'\"]\s*(?:cd|rg|git|gh|node|npm|npx|pnpm|ls|cat|find|fd|grep|sed|awk|curl|python3?|make|"
    r"cargo|echo|wc|head|tail|jq|mv|cp|rm|mkdir|sh|bash|tree|open|lsof|ps|du|df)\b"
    r"|&&|\s\|\s|\s--?[a-z]"
)


def tool_call_tokens(tokens):
    """Indices of the tokens inside the argument list of a `tools.<name>(...)` call."""
    inside = set()
    sig = [i for i, (k, _) in enumerate(tokens) if k not in ("ws", "comment")]
    for n, i in enumerate(sig[:-3]):
        a, b, c, d = (tokens[sig[n + j]] for j in range(4))
        if a == ("word", "tools") and b == ("punct", ".") and c[0] == "word" and d == ("punct", "("):
            depth = 0
            for j in range(sig[n + 3], len(tokens)):
                kind, text = tokens[j]
                if kind == "punct" and text in "([{":
                    depth += 1
                elif kind == "punct" and text in ")]}":
                    depth -= 1
                    if depth == 0:
                        break
                inside.add(j)
    return inside


def redact_code(code, red):
    """Comments dropped (they are free text), literal text redacted, code kept. The text
    of tool-call arguments and of shell commands is replaced entirely: the fake tools
    ignore their arguments, and that text is where private paths, commands, search terms
    and embedded programs live."""
    out = []
    tokens = tokenize(code)
    in_call = tool_call_tokens(tokens)
    for index, (kind, text) in enumerate(tokens):
        if kind in ("string", "template") and (index in in_call or COMMAND_RE.search(text)):
            red.counts["argument-or-command"] += 1
            # Paths, URLs and secrets first, so the report counts them.
            text = red.text(text)
            out.append(red.anon.scrub(text))
            continue
        if kind == "comment":
            red.counts["comment"] += 1
            # Keep line breaks, so line structure stays.
            out.append("\n" * text.count("\n"))
        elif kind in ("string", "template"):
            out.append(red.text(text))
        elif kind == "regex":
            # Only unknown words change: the same input words are renamed the same way
            # in the literals the pattern is matched against.
            out.append(red.anon.regex(text))
        elif kind == "word":
            for u in red.usernames:
                if u in text:
                    red.counts["user-name"] += 1
                    text = text.replace(u, "user")
            out.append(red.anon.identifier(text))
        else:
            out.append(text)
    # Identifiers and code structure are reviewed by hand; flag what looks personal there.
    code = "".join(out)
    code = re.sub(r"[ \t]+\n", "\n", code)
    code = re.sub(r"\n{3,}", "\n\n", code).strip("\n")
    return code


# ---- synthetic tool results ----

WORDS = (
    "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda omega red green blue "
    "north south east west build test check value item entry record status ready done"
).split()


def synthetic_text(rng, lines, style):
    out = []
    for i in range(lines):
        if style == "paths":
            depth = rng.randint(1, 3)
            parts = [rng.choice(["src", "lib", "test", "docs"])] + [f"dir{rng.randint(1, 9)}" for _ in range(depth - 1)]
            out.append("/".join(parts) + f"/file{i + 1}." + rng.choice(["ts", "js", "md", "rs", "json"]))
        elif style == "grep":
            out.append(f"src/file{rng.randint(1, 9)}.ts:{rng.randint(1, 400)}: " + " ".join(rng.choice(WORDS) for _ in range(rng.randint(2, 6))))
        elif style == "code":
            out.append(rng.choice(["const ", "let ", "function ", "export ", "  return ", "  if (", "}"]) + " ".join(rng.choice(WORDS) for _ in range(rng.randint(1, 4))))
        elif style == "markdown":
            out.append(rng.choice(["# ", "## ", "- ", "", "1. "]) + " ".join(rng.choice(WORDS) for _ in range(rng.randint(2, 8))).capitalize())
        else:
            words = [rng.choice(WORDS) for _ in range(rng.randint(2, 8))]
            if rng.random() < 0.3:
                words.append(str(rng.randint(0, 9999)))
            out.append(" ".join(words))
    return "\n".join(out)


def synthetic_json(rng):
    items = []
    for i in range(rng.randint(1, 5)):
        items.append({
            "id": i + 1,
            "name": rng.choice(WORDS),
            "status": rng.choice(["ok", "open", "closed", "pending"]),
            "count": rng.randint(0, 500),
            "tags": [rng.choice(WORDS) for _ in range(rng.randint(0, 3))],
        })
    return json.dumps(items if rng.random() < 0.6 else {"items": items, "total": len(items)}, indent=2)


def synthetic_result(call, rng):
    """A string result of the call's kind. The recorded arguments are read only to pick a
    shape (a file's extension, a command that prints JSON); none of their text is used."""
    name = call["name"]
    try:
        args = json.loads(call.get("args") or "{}")
    except ValueError:
        args = {}
    if not isinstance(args, dict):
        args = {}
    if call.get("status") != "ok":
        return {"ok": False, "value": "Tool failed (synthetic error)"}
    lines = rng.randint(3, 14)
    if name == "read":
        path = str(args.get("path", ""))
        ext = path.rsplit(".", 1)[-1].lower() if "." in path else ""
        if ext == "json":
            return {"ok": True, "value": synthetic_json(rng)}
        if ext in ("md", "txt", "rst"):
            return {"ok": True, "value": synthetic_text(rng, lines, "markdown")}
        if ext in ("ts", "js", "mjs", "tsx", "rs", "py", "go", "c", "h", "swift", "java"):
            return {"ok": True, "value": synthetic_text(rng, lines, "code")}
        return {"ok": True, "value": synthetic_text(rng, lines, "text")}
    if name == "bash":
        cmd = str(args.get("command", ""))
        if re.search(r"--json|\bjq\b|--format[= ]json|-o json", cmd):
            return {"ok": True, "value": synthetic_json(rng)}
        if re.search(r"\b(?:rg|grep)\b", cmd):
            return {"ok": True, "value": synthetic_text(rng, lines, "grep")}
        if re.search(r"\b(?:ls|find|fd|git ls-files|tree)\b", cmd):
            return {"ok": True, "value": synthetic_text(rng, lines, "paths")}
        return {"ok": True, "value": synthetic_text(rng, lines, "text")}
    if name in ("ls", "find"):
        return {"ok": True, "value": synthetic_text(rng, lines, "paths")}
    if name == "grep":
        return {"ok": True, "value": synthetic_text(rng, lines, "grep")}
    return {"ok": True, "value": synthetic_text(rng, lines, "text")}


CASE_TEMPLATE = """\
// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {results};
const fake = Object.create(null);
for (const name of Object.keys(results)) {{
  const queue = results[name];
  fake[name] = async (args) => {{
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : {{ ok: true, value: "" }};
    if (!r.ok) throw new Error(r.value);
    return r.value;
  }};
}}
return await (async (tools) => {{
{code}
}})(fake);
"""


def features(code):
    return {k for k, rx in FEATURE_RES.items() if rx.search(code)}


def shape_key(code):
    """For near-duplicate detection: the code with literals and whitespace erased."""
    try:
        toks = tokenize(code)
    except TokenizeError:
        return None
    out = []
    for kind, text in toks:
        if kind in ("ws", "comment"):
            continue
        out.append({"string": "S", "template": "T", "regex": "R", "number": "N"}.get(kind, text))
    return hashlib.sha256(" ".join(out).encode()).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("extract")
    ap.add_argument("outdir")
    ap.add_argument("--count", type=int, default=150)
    ap.add_argument("--max-len", type=int, default=2000)
    ap.add_argument("--min-len", type=int, default=120)
    ap.add_argument("--exclude", default="", help="file of source ids to leave out (rejected in review)")
    opts = ap.parse_args()
    excluded = set()
    if opts.exclude and os.path.exists(opts.exclude):
        excluded = {l.split()[0] for l in open(opts.exclude) if l.strip() and not l.startswith("#")}

    runs = [json.loads(l) for l in open(opts.extract)]
    usernames = set()
    for r in runs:
        for m in HOME_RE.finditer(r["code"] + " ".join(c.get("args") or "" for c in r["calls"])):
            usernames.add(m.group(1))
    usernames = {u for u in usernames if len(u) >= 3 and u not in ("user", "Shared")}

    stats = collections.Counter()
    pool = {}
    for r in runs:
        stats["executions"] += 1
        code = r["code"]
        names = {c["name"] for c in r["calls"]}
        if not names <= STRING_TOOLS:
            stats["other tools (mcp, models, edit, write, ...)"] += 1
            continue
        if EXCLUDED_GLOBALS.search(code):
            stats["unfaked globals or modules"] += 1
            continue
        if not opts.min_len <= len(code) <= opts.max_len:
            stats["length outside range"] += 1
            continue
        if r["source"] in excluded:
            stats["excluded (a SyntaxError in qjs, or rejected in review)"] += 1
            continue
        key = shape_key(code)
        if key is None:
            stats["does not tokenize"] += 1
            continue
        if key in pool:
            stats["duplicate"] += 1
            continue
        pool[key] = r

    # Greedy selection: each pick maximizes the features it adds (rare features count
    # more), with a small preference for shorter scripts and for tool sets seen less.
    candidates = list(pool.values())
    feats = {id(r): features(r["code"]) for r in candidates}
    feat_freq = collections.Counter(f for r in candidates for f in feats[id(r)])
    covered = collections.Counter()
    toolsets = collections.Counter()
    chosen = []
    while candidates and len(chosen) < opts.count:
        def score(r):
            fs = feats[id(r)]
            gain = sum(1.0 / (1 + covered[f]) / math.sqrt(feat_freq[f]) for f in fs)
            ts = tuple(sorted({c["name"] for c in r["calls"]}))
            gain += 0.5 / (1 + toolsets[ts])
            return gain - len(r["code"]) / 20000
        best = max(candidates, key=score)
        candidates.remove(best)
        chosen.append(best)
        for f in feats[id(best)]:
            covered[f] += 1
        toolsets[tuple(sorted({c["name"] for c in best["calls"]}))] += 1

    os.makedirs(opts.outdir, exist_ok=True)
    report = []
    written = 0
    for i, r in enumerate(chosen):
        case_id = f"h{i + 1:03d}"
        red = Redactor(case_id, usernames)
        try:
            code = redact_code(r["code"], red)
        except TokenizeError:
            stats["does not tokenize"] += 1
            continue
        rng = random.Random(case_id)
        results = collections.defaultdict(list)
        for c in r["calls"]:
            results[c["name"]].append(synthetic_result(c, rng))
        body = CASE_TEMPLATE.format(results=json.dumps(dict(results), indent=1), code=code)
        name = f"{case_id}-{'-'.join(sorted(results)) or 'no-tools'}.js"
        with open(os.path.join(opts.outdir, name), "w") as f:
            f.write(body)
        written += 1
        if red.anon.count:
            red.counts["word"] = red.anon.count
        counts = ", ".join(f"{k} {v}" for k, v in sorted(red.counts.items()) if v) or "nothing"
        flags = ", ".join(f"{k} {v}" for k, v in sorted(red.flags.items()))
        calls = collections.Counter(c["name"] for c in r["calls"])
        report.append((name, r["source"], counts, flags, dict(calls), sorted(feats[id(r)])))

    with open(os.path.join(opts.outdir, "REDACTION.md"), "w") as f:
        f.write("# Harvested codemode cases: redaction report\n\n")
        f.write(
            "Generated by `scripts/harvest/build.py` (0118, D2280). Per case: what the automated pass\n"
            "redacted, by kind and count (never the values), what it flagged for review, and the\n"
            "synthetic tool calls. Every case was also reviewed by hand before it was committed.\n"
            "`source` is a hash of the session file and the execution's index in it; it identifies\n"
            "the origin for whoever has the sessions and reveals nothing else.\n\n"
        )
        f.write("Kinds: comment (dropped), argument-or-command (tool-call argument or shell command\n")
        f.write("text, replaced entirely), path (components renamed), home-dir, user-name, host,\n")
        f.write("url, email, ip, key, token, connection-string, high-entropy (replaced by synthetic\n")
        f.write("text of the same shape), word (a word part that is not a common English word or a\n")
        f.write("programming term, in a literal or an identifier, renamed consistently, D2281).\n\n")
        f.write("| case | source | redacted | flagged | tool calls |\n| --- | --- | --- | --- | --- |\n")
        for name, source, counts, flags, calls, _ in report:
            call_text = ", ".join(f"{k} {v}" for k, v in sorted(calls.items())) or "none"
            f.write(f"| {name} | {source} | {counts} | {flags or '-'} | {call_text} |\n")
        f.write("\n## Selection\n\n")
        for k, v in stats.items():
            f.write(f"- {k}: {v}\n")
        f.write(f"- candidates after filters and deduplication: {len(pool)}\n")
        f.write(f"- cases written: {written}\n")
        f.write("\n## Feature coverage\n\n")
        cov = collections.Counter(f for *_, fs in report for f in fs)
        f.write(", ".join(f"{k} {cov[k]}" for k in FEATURES) + "\n")
    print(f"build: {written} cases from {len(pool)} candidates ({dict(stats)})", file=sys.stderr)


if __name__ == "__main__":
    main()
