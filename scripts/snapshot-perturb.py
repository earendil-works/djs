#!/usr/bin/env python3
"""The perturbations of scripts/snapshot-perturb.sh (0302, D3712), applied to the builtin
sources of the tree given: record fields in another order, state constants with other
values. Names stay, so a snapshot by name must continue the same."""
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])


def edit(rel, f):
    p = root / rel
    s = p.read_text()
    t = f(s)
    if t == s:
        sys.exit(f"snapshot-perturb.py: {rel}: nothing changed")
    p.write_text(t)


def reverse_enum(name):
    """Reverses the members of `enum name` (a record schema: its slots move)."""

    def f(s):
        m = re.search(r"(enum " + name + r" \{\n)(.*?)(\n\})", s, re.S)
        members = [l for l in m.group(2).split("\n") if l.strip()]
        # A member is its doc comment lines and its name line.
        groups, cur = [], []
        for l in members:
            cur.append(l)
            if not l.strip().startswith(("/**", "*", "*/")):
                groups.append(cur)
                cur = []
        body = "\n".join(l for g in reversed(groups) for l in g)
        return s[: m.start(2)] + body + s[m.end(2) :]

    return f


def values(name, new):
    """Gives the members of constant set `name` the values `new` (member -> value)."""

    def f(s):
        m = re.search(r"(enum " + name + r" \{\n)(.*?)(\n\})", s, re.S)
        body = m.group(2)
        for k, v in new.items():
            body, n = re.subn(r"(\b" + k + r" = )-?\d+", rf"\g<1>{v}", body)
            assert n == 1, (name, k)
        return s[: m.start(2)] + body + s[m.end(2) :]

    return f


edit("builtins/promise.ts", reverse_enum("ElementRecord"))
edit("builtins/promise.ts", reverse_enum("Capability"))
edit("builtins/promise.ts", reverse_enum("CombinatorRecord"))
edit("builtins/asyncgen.ts", reverse_enum("AsyncGeneratorRequest"))
edit("builtins/fromasync.ts", reverse_enum("FromAsyncState"))
edit("builtins/module.ts", reverse_enum("ModuleRecord"))
edit("builtins/intrinsics.d.ts", values("PromiseStates", {"fulfilled": 2, "rejected": 1}))
edit("builtins/intrinsics.d.ts", values("PromiseFlags", {"handled": 2, "resolved": 1}))
edit("builtins/intrinsics.d.ts", values("GeneratorStates", {"suspendedYield": 4, "awaiting": 1}))
edit("builtins/iterator.ts", values("HelperStates", {"suspendedYield": 3, "completed": 1}))
edit("builtins/disposable.ts", values("DisposableStates", {"pending": 1, "disposed": 0}))
