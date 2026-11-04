#!/usr/bin/env python3
"""Scripted three-way merges through the merge drivers (0214): a throwaway repository
with this checkout's .gitattributes and scripts/merge-driver.py, a base commit, two
branches that change the same derived files, and `git merge`. No real branches of the
project are touched.

    python3 scripts/merge-driver-test.py

Checks:
- tests/smap/summary.txt (`djs-generated`): changed on both sides, the merge succeeds,
  keeps one side and records the path in $GIT_DIR/djs-regen for scripts/regen.sh;
- tests/scores/dump-size.tsv (`djs-scores`): rows added on both sides next to each other,
  a row changed on one side, a row changed on both (the smaller number per column), a row
  deleted on one side and unchanged on the other, a row deleted on one side and changed on
  the other; the result is sorted by suite (first appearance) and case;
- a score that does not parse falls back to a line merge, which conflicts.
"""

import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HEAD = "# The plain dump size score\n"


def git(repo, *args, check=True):
    r = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed:\n{r.stdout}{r.stderr}")
    return r


def write(repo, path, text):
    p = os.path.join(repo, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)


def read(repo, path):
    with open(os.path.join(repo, path), encoding="utf-8") as f:
        return f.read()


def score(rows):
    return HEAD + "".join(f"{s}\t{c}\t{r}\t{g}\n" for s, c, r, g in rows)


def setup(tmp):
    repo = os.path.join(tmp, "repo")
    os.makedirs(os.path.join(repo, "scripts"))
    git(repo, "init", "-q", "-b", "main")
    git(repo, "config", "user.email", "test@example.invalid")
    git(repo, "config", "user.name", "merge-driver-test")
    shutil.copy(os.path.join(ROOT, ".gitattributes"), repo)
    shutil.copy(os.path.join(ROOT, "scripts/merge-driver.py"), os.path.join(repo, "scripts"))
    # The drivers as scripts/setup-merge-drivers.sh registers them, in this repository.
    with open(os.path.join(ROOT, "scripts/setup-merge-drivers.sh"), encoding="utf-8") as f:
        for line in f:
            if line.startswith("git config "):
                args = line.split(" ", 3)[2:]
                key, value = args[0], args[1].strip().strip('"')
                git(repo, "config", key, value)
    return repo


def branch(repo, name, files):
    git(repo, "checkout", "-q", "-b", name, "main")
    for path, text in files.items():
        if text is None:
            os.remove(os.path.join(repo, path))
        else:
            write(repo, path, text)
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", name)
    git(repo, "checkout", "-q", "main")


def main():
    failures = []

    def expect(what, got, want):
        if got != want:
            failures.append(f"{what}:\n--- got\n{got}--- want\n{want}")

    tmp = tempfile.mkdtemp(prefix="merge-driver-test-")
    try:
        repo = setup(tmp)
        summary = "tests/smap/summary.txt"
        dsz = "tests/scores/dump-size.tsv"
        base_rows = [
            ("corpus", "a.js", 100, 50),
            ("corpus", "c.js", 300, 150),
            ("corpus", "e.js", 500, 250),
            ("corpus", "g.js", 700, 350),
            ("corpus", "h.js", 800, 400),
            ("v2", "loops", 10, 5),
        ]
        write(repo, summary, "# Suspension map coverage\nsites 10\n")
        write(repo, dsz, score(base_rows))
        git(repo, "add", "-A")
        git(repo, "commit", "-q", "-m", "base")

        branch(repo, "ours", {
            summary: "# Suspension map coverage\nsites 11\n",
            dsz: score([
                ("corpus", "a.js", 90, 50),     # changed here only
                ("corpus", "b.js", 200, 100),   # added here, next to theirs' d.js
                ("corpus", "c.js", 310, 140),   # changed on both sides
                ("corpus", "g.js", 700, 350),   # e.js deleted here, unchanged there
                ("corpus", "h.js", 800, 400),   # deleted there, unchanged here
                ("v2", "loops", 10, 5),
            ]),
        })
        branch(repo, "theirs", {
            summary: "# Suspension map coverage\nsites 12\n",
            dsz: score([
                ("corpus", "a.js", 100, 50),
                ("corpus", "c.js", 290, 160),
                ("corpus", "d.js", 400, 200),   # added here
                ("corpus", "e.js", 500, 250),
                ("corpus", "g.js", 690, 340),   # changed here only
                ("v2", "loops", 10, 5),
                ("v2", "zeta", 20, 10),         # added at a suite's end
            ]),
        })
        git(repo, "checkout", "-q", "ours")
        r = git(repo, "merge", "-q", "--no-edit", "theirs", check=False)
        if r.returncode != 0:
            failures.append(f"the merge conflicted:\n{r.stdout}{r.stderr}")
            git(repo, "merge", "--abort", check=False)
        else:
            expect("summary: our side", read(repo, summary), "# Suspension map coverage\nsites 11\n")
            marker = git(repo, "rev-parse", "--git-path", "djs-regen").stdout.strip()
            marker = os.path.join(repo, marker) if not os.path.isabs(marker) else marker
            got = open(marker, encoding="utf-8").read() if os.path.exists(marker) else ""
            expect("summary: marked for regen.sh", got, summary + "\n")
            expect("dump-size merged by row", read(repo, dsz), score([
                ("corpus", "a.js", 90, 50),
                ("corpus", "b.js", 200, 100),
                ("corpus", "c.js", 290, 140),
                ("corpus", "d.js", 400, 200),
                ("corpus", "g.js", 690, 340),
                ("v2", "loops", 10, 5),
                ("v2", "zeta", 20, 10),
            ]))

        # Deleted on one side, changed on the other: the change stays.
        git(repo, "checkout", "-q", "main")
        branch(repo, "del", {dsz: score([r for r in base_rows if r[1] != "a.js"])})
        branch(repo, "chg", {dsz: score([("corpus", "a.js", 120, 60)] + base_rows[1:])})
        git(repo, "checkout", "-q", "del")
        r = git(repo, "merge", "-q", "--no-edit", "chg", check=False)
        if r.returncode != 0:
            failures.append(f"delete/change conflicted:\n{r.stdout}{r.stderr}")
            git(repo, "merge", "--abort", check=False)
        else:
            expect("delete/change keeps the change", read(repo, dsz),
                   score([("corpus", "a.js", 120, 60)] + base_rows[1:]))

        # A score that does not parse: a line merge, which conflicts here.
        git(repo, "checkout", "-q", "main")
        branch(repo, "bad1", {dsz: score(base_rows) + "garbage line\n"})
        branch(repo, "bad2", {dsz: score(base_rows) + "other garbage\n"})
        git(repo, "checkout", "-q", "bad1")
        r = git(repo, "merge", "-q", "--no-edit", "bad2", check=False)
        if r.returncode == 0:
            failures.append("a score that does not parse merged without a conflict")
        elif "<<<<<<<" not in read(repo, dsz):
            failures.append("a score that does not parse: no conflict markers")
        git(repo, "merge", "--abort", check=False)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    if failures:
        print("merge drivers: FAILED\n" + "\n".join(failures))
        return 1
    print("merge drivers: OK (smap summary kept and marked; dump-size rows merged by key)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
