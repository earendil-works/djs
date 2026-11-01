#!/usr/bin/env python3
"""Step 1 of the codemode corpus harvest (0118, D2280): extract codemode executions.

Streams pi session files (JSONL, one entry per line) and writes one JSON line per
codemode execution to OUT: the script, the nested calls it made (name, status, args)
and whether the run failed. Session data is private: OUT must stay in
work/scratch/0118/ (gitignored), and nothing here prints session content.

    scripts/harvest/extract.py [SESSIONS_DIR] OUT
"""
import hashlib
import json
import os
import sys


def session_files(root):
    for dirpath, _, names in os.walk(root):
        for name in names:
            if name.endswith(".jsonl"):
                yield os.path.join(dirpath, name)


def executions(path):
    """The codemode executions of one session file, in order."""
    pending = {}
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            if '"codemode"' not in line:
                continue
            try:
                entry = json.loads(line)
            except ValueError:
                continue
            m = entry.get("message") or {}
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
                yield {"code": code, "calls": calls, "isError": bool(m.get("isError"))}


def main():
    root = sys.argv[1] if len(sys.argv) > 2 else os.path.expanduser("~/.pi/agent/sessions")
    out_path = sys.argv[-1]
    n_files = n_runs = 0
    with open(out_path, "w") as out:
        for path in session_files(root):
            n_files += 1
            # A cheap pre-filter: most session files never ran codemode.
            with open(path, "rb") as f:
                if b'"codemode"' not in f.read():
                    continue
            session = hashlib.sha256(path.encode()).hexdigest()[:12]
            for i, run in enumerate(executions(path)):
                run["source"] = f"{session}:{i}"
                out.write(json.dumps(run) + "\n")
                n_runs += 1
    print(f"extract: {n_runs} codemode executions from {n_files} session files", file=sys.stderr)


if __name__ == "__main__":
    main()
