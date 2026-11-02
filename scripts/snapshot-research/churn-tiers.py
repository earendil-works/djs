# Which snapshot tier would survive each merged issue (0142, docs/goldilocks-snapshots.md)?
# Groups main's commits by issue as churn.py does and checks what each issue touched:
# - image (today): any change to djs-core's sources or builtins (the build hash);
# - image with symbolic relocations: the formats a raw session image keeps inside it
#   (cell layouts, well-known atoms, opcodes, object and shape representation; the
#   collector's layout rules are in cells.rs) - builtins, intrinsic ids and the base may move;
# - by meaning: none of those; what breaks it is a change of behaviour or of a builtin
#   record schema (not detectable from file names; see the report).
import re, subprocess, collections
log = subprocess.run(["git", "log", "--reverse", "--format=@@%h %ad %s", "--date=short", "--name-only", "main"],
                     capture_output=True, text=True).stdout
issues = collections.OrderedDict()
cur = None
for line in log.splitlines():
    if line.startswith("@@"):
        h, date, subj = line[2:].split(" ", 2)
        m = re.match(r"(\d{4}):", subj)
        cur = m.group(1) if m else None
        if cur and cur not in issues:
            issues[cur] = set()
    elif line.strip() and cur:
        issues[cur].add(line.strip())

def core(f):
    return f.startswith("crates/djs-core/src/") and f.endswith(".rs") and not f.endswith("tests.rs") and "/tests/" not in f
image = lambda f: core(f) or f == "builtins/gen/prelude.js"
reloc = lambda f: f in ("crates/djs-core/src/heap/cells.rs", "crates/djs-core/src/heap/atom.rs",
                        "crates/djs-core/src/bytecode/ops.rs", "crates/djs-core/src/heap/shape.rs",
                        "crates/djs-core/src/heap/object.rs")
strict = lambda f: f in ("crates/djs-core/src/heap/cells.rs", "crates/djs-core/src/heap/atom.rs",
                         "crates/djs-core/src/bytecode/ops.rs")
keys = list(issues)
for title, sel in [("all", keys), ("last 30", keys[-30:])]:
    n = len(sel)
    a = sum(any(image(f) for f in issues[k]) for k in sel)
    b = sum(any(reloc(f) for f in issues[k]) for k in sel)
    c = sum(any(strict(f) for f in issues[k]) for k in sel)
    print(f"{title} ({n} issues): breaks today's image {a} ({100*a/n:.0f}%); "
          f"an image with symbolic relocations: {c} ({100*c/n:.0f}%) change the layout table, "
          f"the well-known atoms or the opcodes, {b} ({100*b/n:.0f}%) with object/shape code")

# A release bundles many issues: the share of windows of N consecutive issues that leave
# each kind of snapshot intact.
for n in (5, 10, 30):
    wins = [keys[i:i + n] for i in range(len(keys) - n + 1)]
    for name, pred in (("today's image", image), ("relocations (strict)", strict), ("relocations (with object code)", reloc)):
        ok = sum(not any(pred(f) for k in w for f in issues[k]) for w in wins)
        print(f"  releases of {n:2} issues: {name} survives {ok} of {len(wins)} ({100*ok/len(wins):.0f}%)")
    last = keys[-n:]
    print(f"    (the last {n}: image {'survives' if not any(image(f) for k in last for f in issues[k]) else 'breaks'}, "
          f"strict relocations {'survive' if not any(strict(f) for k in last for f in issues[k]) else 'break'})")
