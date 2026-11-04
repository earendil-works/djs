#!/usr/bin/env python3
"""Git merge drivers for generated files (0079, D1791).

    scripts/merge-driver.py generated  %O %A %B %P
    scripts/merge-driver.py intrinsics %O %A %B %P
    scripts/merge-driver.py scores     %O %A %B %P

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

`scores`: tests/scores/dump-size.tsv, one `SUITE CASE RAW GZIP` row per case (D2906).
Rows merge three-way by key (SUITE CASE), so two branches that add or change different
cases never conflict, even on adjacent lines: a row changed or added on one side takes
that side's numbers; one changed (or added) on both takes the smaller number per column
(the score is a ratchet, and the gate checks the merged result); one deleted on one side
and unchanged on the other is deleted (one deleted on one side and changed on the other
stays, a stale line the gate only reports). The comment header is ours unless only theirs
changed it. Suites keep the order they first appear in, cases are sorted within a suite,
as `dump-size.ts --update` writes them. A file that does not parse is merged by
`git merge-file`, conflicts and all (0214).
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


def read_score(path):
    """(header lines, {(suite, case): (raw, gzip)}, suite order); None if a row does not
    parse."""
    header, rows, suites = [], {}, []
    with open(path, encoding="utf-8") as f:
        for line in f.read().split("\n"):
            if not line:
                continue
            if line.startswith("#"):
                header.append(line)
                continue
            parts = line.split("\t")
            if len(parts) != 4 or not parts[2].isdigit() or not parts[3].isdigit():
                return None
            key = (parts[0], parts[1])
            if key in rows:
                return None
            rows[key] = (int(parts[2]), int(parts[3]))
            if parts[0] not in suites:
                suites.append(parts[0])
    return header, rows, suites


def merge_scores(base, ours, theirs):
    """The merged score's text, or None if a side does not parse."""
    b, o, t = read_score(base), read_score(ours), read_score(theirs)
    if b is None or o is None or t is None:
        return None
    (bh, br, bs), (oh, orows, osuites), (th, trows, tsuites) = b, o, t
    header = th if oh == bh else oh
    merged = {}
    for key in set(br) | set(orows) | set(trows):
        bv, ov, tv = br.get(key), orows.get(key), trows.get(key)
        if ov == bv:
            v = tv
        elif tv == bv:
            v = ov
        elif ov is None or tv is None:
            # Deleted on one side, changed on the other: the change stays.
            v = ov if tv is None else tv
        else:
            v = (min(ov[0], tv[0]), min(ov[1], tv[1]))
        if v is not None:
            merged[key] = v
    order = []
    for s in osuites + tsuites + bs:
        if s not in order:
            order.append(s)
    lines = list(header)
    for s in order:
        for key in sorted(k for k in merged if k[0] == s):
            raw, gz = merged[key]
            lines.append(f"{s}\t{key[1]}\t{raw}\t{gz}")
    return "\n".join(lines) + "\n"


def scores(base, ours, theirs):
    text = merge_scores(base, ours, theirs)
    if text is None:
        merged = subprocess.run(
            ["git", "merge-file", "-p", "-L", "ours", "-L", "base", "-L", "theirs", ours, base, theirs],
            capture_output=True, text=True,
        )
        with open(ours, "w", encoding="utf-8") as f:
            f.write(merged.stdout)
        return 0 if merged.returncode == 0 else 1
    with open(ours, "w", encoding="utf-8") as f:
        f.write(text)
    return 0


def main():
    mode, base, ours, theirs, path = sys.argv[1:6]
    if mode == "generated":
        mark(path)
        return 0
    if mode == "intrinsics":
        return intrinsics(base, ours, theirs, path)
    if mode == "scores":
        return scores(base, ours, theirs)
    print(f"merge-driver.py: unknown mode {mode}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
