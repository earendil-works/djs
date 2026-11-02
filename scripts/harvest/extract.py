#!/usr/bin/env python3
"""Step 1 of the codemode corpus harvest (0118, D2280): extract codemode executions.

Streams pi session files (JSONL, one entry per line) and writes one JSON line per
codemode execution to OUT: the script, the nested calls it made (name, status, args)
and whether the run failed. Session data is private: OUT must stay in
work/scratch/0118/ (gitignored), and nothing here prints session content.

    scripts/harvest/extract.py [SESSIONS_DIR] OUT

SESSIONS_DIR may also hold p2 session databases (`*.sqlite`, the p2 app's
~/.pi/p2/sessions, pi-durable's schema; 0149). Copy them first (with their -wal/-shm
files) to work/scratch/: they are opened read-only, and only their `entries` table is
read. Nothing else under ~/.pi/p2 may be opened (credentials and keys live there).
"""
import datetime
import hashlib
import json
import os
import re
import sys


def session_files(root):
    for dirpath, _, names in os.walk(root):
        for name in names:
            if name.endswith(".jsonl") or name.endswith(".sqlite"):
                yield os.path.join(dirpath, name)


def p2_messages(path):
    """The messages of a p2 session database (pi-durable's `entries`: one record per
    entry, its messages under `model`), in commit order per conversation, as the
    (entry, message) pairs a pi JSONL session gives."""
    import sqlite3

    db = sqlite3.connect(f"file:{os.path.abspath(path)}?mode=ro", uri=True)
    try:
        rows = db.execute("SELECT record FROM entries ORDER BY conversation_id, id").fetchall()
    finally:
        db.close()
    for (record,) in rows:
        try:
            entry = json.loads(record)
        except ValueError:
            continue
        if entry.get("kind") not in ("pi.assistant", "pi.tool-result"):
            continue
        for m in entry.get("model") or []:
            if isinstance(m, dict):
                ts = m.get("timestamp")
                if isinstance(ts, (int, float)):
                    ts = datetime.datetime.fromtimestamp(ts / 1000, datetime.timezone.utc).isoformat()
                yield {"timestamp": ts}, m


def jsonl_messages(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            if '"codemode' not in line:
                continue
            try:
                entry = json.loads(line)
            except ValueError:
                continue
            yield entry, entry.get("message") or {}


LOAD_KEY_RE = re.compile(r"""\bload\(\s*(["'])((?:(?!\1)[^\\])*)\1\s*\)""")


def declarations(description):
    """Tool declarations in the codemode tool's description: `name(args: ...): Promise<T>`
    by the tool's JavaScript name, as the type text T (0121, D2283)."""
    decls = {}
    for m in re.finditer(r"([A-Za-z_$][\w$]*)\(args: ", description):
        i = m.end()
        depth = 0
        # Skip the argument type: up to the `)` that closes the parameter list.
        while i < len(description):
            c = description[i]
            if c in "({[<":
                depth += 1
            elif c in ")}]>":
                if depth == 0:
                    break
                depth -= 1
            elif description.startswith("//", i):
                i = description.find("\n", i)
                if i < 0:
                    break
                continue
            i += 1
        if i < 0 or not description.startswith("): Promise<", i):
            continue
        j = i + len("): Promise<")
        depth = 0
        k = j
        while k < len(description):
            c = description[k]
            if description.startswith("//", k):
                k = description.find("\n", k)
                if k < 0:
                    break
                continue
            if c in "({[<":
                depth += 1
            elif c in ")}]":
                depth -= 1
            elif c == ">":
                if depth == 0:
                    decls[m.group(1)] = description[j:k]
                    break
                depth -= 1
            k += 1
    return decls


def executions(path):
    """The codemode executions of one session file, in order, with the tool declarations
    the codemode tool showed at the time and the stored values (`codemode-store`) of the
    keys the script loads."""
    pending = {}
    decls = {}
    store = {}
    messages = p2_messages(path) if path.endswith(".sqlite") else jsonl_messages(path)
    for entry, m in messages:
        if entry.get("type") == "custom" and entry.get("customType") == "codemode-store":
            data = entry.get("data") or {}
            for k in data.get("delete") or []:
                store.pop(k, None)
            for k, v in (data.get("set") or {}).items():
                store[k] = v
            continue
        # Declarations come with the tools (the codemode tool's catalog, and each
        # tool's own sample) and from describeTool() output in earlier scripts.
        for tool in m.get("toolsAdded") or []:
            if isinstance(tool, dict):
                decls.update(declarations(tool.get("description") or ""))
        if m.get("role") == "toolResult" and m.get("toolName") == "codemode":
            for c in m.get("content") or []:
                if isinstance(c, dict) and isinstance(c.get("text"), str) and "): Promise<" in c["text"]:
                    decls.update(declarations(c["text"]))
        if m.get("role") == "assistant":
            for c in m.get("content") or []:
                if isinstance(c, dict) and c.get("type") == "toolCall" and c.get("name") == "codemode":
                    code = (c.get("arguments") or {}).get("code")
                    if isinstance(code, str):
                        pending[c.get("id")] = code
        elif m.get("role") == "toolResult" and m.get("toolName") == "codemode":
            code = pending.pop(m.get("toolCallId"), None)
            if code is None:
                continue
            details = m.get("details") or {}
            calls = []
            for c in details.get("calls") or []:
                args = c.get("args")
                if not isinstance(args, str):
                    args = json.dumps(args)
                calls.append({"name": c.get("name"), "status": c.get("status"), "args": args})
            called = {toident(c["name"] or "") for c in calls}
            loads = {k: store[k] for _, k in LOAD_KEY_RE.findall(code) if k in store}
            yield {
                "code": code,
                "calls": calls,
                "isError": bool(m.get("isError")),
                "decls": {k: v for k, v in decls.items() if k in called},
                "loads": loads,
                # When the run finished (the result entry's time), for usage over time.
                "timestamp": entry.get("timestamp") or m.get("timestamp"),
            }


def toident(name):
    """pi's toCodemodeIdentifier: invalid identifier characters become `_`."""
    out = ""
    for ch in name:
        ok = re.match(r"[A-Za-z_$]", ch) if not out else re.match(r"[A-Za-z0-9_$]", ch)
        out += ch if ok else "_"
    return out or "_"


def main():
    root = sys.argv[1] if len(sys.argv) > 2 else os.path.expanduser("~/.pi/agent/sessions")
    out_path = sys.argv[-1]
    n_files = n_runs = 0
    with open(out_path, "w") as out:
        for path in session_files(root):
            n_files += 1
            # A cheap pre-filter: most session files never ran codemode.
            with open(path, "rb") as f:
                if not path.endswith(".sqlite") and b'"codemode"' not in f.read():
                    continue
            session = hashlib.sha256(path.encode()).hexdigest()[:12]
            if path.endswith(".sqlite"):
                session = "p2-" + session
            for i, run in enumerate(executions(path)):
                run["source"] = f"{session}:{i}"
                out.write(json.dumps(run) + "\n")
                n_runs += 1
    print(f"extract: {n_runs} codemode executions from {n_files} session files", file=sys.stderr)


if __name__ == "__main__":
    main()
