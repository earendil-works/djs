#!/usr/bin/env python3
"""Disassembles the codemode corpus with every built CLI (work/scratch/0299/bins/<sha>/djs).

Each case is compiled as difftest compiles it: a Script holding the arrow
`(async (tools, console) => {SOURCE\n})`. The corpus is today's, copied once to
work/scratch/0299/corpus, so every build compiles the same text. Output:
work/scratch/0299/disasm/<sha>/<case>.txt, or <case>.err when the build cannot compile it.
"""
import os, subprocess, sys, glob, shutil
from concurrent.futures import ThreadPoolExecutor

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
S = os.path.join(ROOT, "work/scratch/0299")
CORPUS = os.path.join(S, "corpus")

def prepare():
    if os.path.isdir(CORPUS):
        return
    os.makedirs(CORPUS)
    for area in ("codemode", "codemode-harvest"):
        for p in sorted(glob.glob(os.path.join(ROOT, "tests/corpus", area, "*.js"))):
            src = open(p, encoding="utf-8").read()
            name = area + "__" + os.path.basename(p)
            with open(os.path.join(CORPUS, name), "w", encoding="utf-8") as f:
                f.write("(async (tools, console) => {" + src + "\n})")

def one(binary, case, outdir):
    out = os.path.join(outdir, case[:-3] + ".txt")
    err = os.path.join(outdir, case[:-3] + ".err")
    if os.path.exists(out) or os.path.exists(err):
        return
    r = subprocess.run(["nice", "-n", "10", binary, "disasm", os.path.join(CORPUS, case)],
                       capture_output=True, timeout=60)
    if r.returncode == 0:
        open(out, "wb").write(r.stdout)
    else:
        open(err, "wb").write(r.stdout + r.stderr)

def main():
    prepare()
    cases = sorted(os.listdir(CORPUS))
    shas = sys.argv[1:] or sorted(os.listdir(os.path.join(S, "bins")))
    with ThreadPoolExecutor(4) as pool:
        for sha in shas:
            b = os.path.join(S, "bins", sha, "djs")
            if not os.path.exists(b):
                continue
            outdir = os.path.join(S, "disasm", sha)
            os.makedirs(outdir, exist_ok=True)
            list(pool.map(lambda c: one(b, c, outdir), cases))
            print(sha, len(os.listdir(outdir)), flush=True)

main()
