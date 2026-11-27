#!/usr/bin/env python3
"""Scripted three-way merges through the merge drivers (0214): a throwaway repository
with this checkout's .gitattributes and scripts/merge-driver.py, a base commit, two
branches that change the same derived file, and `git merge`. No real branches of the
project are touched.

    python3 scripts/merge-driver-test.py

Checks: tests/builtins/registry.tsv (`djs-generated`), changed on both sides: the merge
succeeds, keeps one side and records the path in $GIT_DIR/djs-regen for scripts/regen.sh.
"""

import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


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
        reg = "tests/builtins/registry.tsv"
        write(repo, reg, "# registry\n1\ta\n")
        git(repo, "add", "-A")
        git(repo, "commit", "-q", "-m", "base")
        branch(repo, "ours", {reg: "# registry\n1\ta\n2\tb\n"})
        branch(repo, "theirs", {reg: "# registry\n1\ta\n2\tc\n"})
        git(repo, "checkout", "-q", "ours")
        r = git(repo, "merge", "-q", "--no-edit", "theirs", check=False)
        if r.returncode != 0:
            failures.append(f"the merge conflicted:\n{r.stdout}{r.stderr}")
            git(repo, "merge", "--abort", check=False)
        else:
            expect("registry: our side", read(repo, reg), "# registry\n1\ta\n2\tb\n")
            marker = git(repo, "rev-parse", "--git-path", "djs-regen").stdout.strip()
            marker = os.path.join(repo, marker) if not os.path.isabs(marker) else marker
            got = open(marker, encoding="utf-8").read() if os.path.exists(marker) else ""
            expect("registry: marked for regen.sh", got, reg + "\n")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    if failures:
        print("merge drivers: FAILED\n" + "\n".join(failures))
        return 1
    print("merge drivers: OK (a generated file kept and marked)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
