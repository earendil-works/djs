#!/usr/bin/env python3
"""Git merge drivers for generated files (0079, D1791).

    scripts/merge-driver.py generated  %O %A %B %P
    scripts/merge-driver.py intrinsics %O %A %B %P

Registered by `scripts/setup-merge-drivers.sh` (merge-to-main.sh runs it) and selected
in `.gitattributes`. Git calls a driver only when both sides changed a file.

`generated`: files written by a generator (builtins/gen/*, js/src/layout.generated.ts).
Keeps our side (%A, unchanged) and records the file in the worktree's
`$GIT_DIR/djs-regen`; `scripts/regen.sh` then rewrites it from the merged sources, so a
merge of generated text never has to be resolved by hand.

`intrinsics`: crates/djs-core/src/bytecode/intrinsics.rs, whose `INTRINSICS` table is
generated from builtins/ (the `table_matches_sources` test) but lives next to hand-written
code. A normal three-way merge; if every conflict lies inside the table, the conflicts
take our side and the table is regenerated like a generated file. Conflicts elsewhere are
left in place (exit 1), as without the driver.
"""

import re
import subprocess
import sys


def mark(path):
    marker = subprocess.run(
        ["git", "rev-parse", "--git-path", "djs-regen"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    with open(marker, "a", encoding="utf-8") as f:
        f.write(path + "\n")


TABLE_ENTRY = re.compile(r"^    i\(\"\w+\", [LRB], \d+, (true|false)\),$")


def only_table_conflicts(text):
    """Whether every line inside every conflict block is an INTRINSICS table entry."""
    inside = False
    for line in text.split("\n"):
        if line.startswith("<<<<<<< "):
            inside = True
        elif line.startswith(">>>>>>> "):
            inside = False
        elif inside and line not in ("=======",) and not line.startswith("||||||| "):
            if not TABLE_ENTRY.match(line):
                return False
    return True


def intrinsics(base, ours, theirs, path):
    merged = subprocess.run(
        ["git", "merge-file", "-p", "-L", "ours", "-L", "base", "-L", "theirs", ours, base, theirs],
        capture_output=True, text=True,
    )
    if merged.returncode == 0:
        with open(ours, "w", encoding="utf-8") as f:
            f.write(merged.stdout)
        return 0
    text = merged.stdout
    if not only_table_conflicts(text):
        # A conflict outside the table: hand it to the user like a normal merge.
        with open(ours, "w", encoding="utf-8") as f:
            f.write(text)
        return 1
    resolved = subprocess.run(
        ["git", "merge-file", "-p", "--ours", ours, base, theirs], capture_output=True, text=True
    ).stdout
    with open(ours, "w", encoding="utf-8") as f:
        f.write(resolved)
    mark(path)
    return 0


def main():
    mode, base, ours, theirs, path = sys.argv[1:6]
    if mode == "generated":
        mark(path)
        return 0
    if mode == "intrinsics":
        return intrinsics(base, ours, theirs, path)
    print(f"merge-driver.py: unknown mode {mode}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
