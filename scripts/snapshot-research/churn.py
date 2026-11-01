# How often did each input of a snapshot change (0140)? Groups main's commits by issue (the
# "NNNN:" prefix: one merge each) and checks which paths each issue touched.
import re, subprocess, collections, sys
log = subprocess.run(["git", "log", "--reverse", "--format=@@%h %ad %s", "--date=short", "--name-only", "main"],
                     capture_output=True, text=True).stdout
issues = collections.OrderedDict()
cur = None
for line in log.splitlines():
    if line.startswith("@@"):
        h, date, subj = line[2:].split(" ", 2)
        m = re.match(r"(\d{4}):", subj)
        key = m.group(1) if m else None
        cur = key
        if key and key not in issues:
            issues[key] = {"date": date, "files": set(), "commits": 0}
        if key:
            issues[key]["commits"] += 1
    elif line.strip() and cur:
        issues[cur]["files"].add(line.strip())

def core_src(f):
    return f.startswith("crates/djs-core/src/") and f.endswith(".rs") and not f.endswith("tests.rs") and "/tests/" not in f
inputs = collections.OrderedDict([
    ("build hash (djs-core sources, prelude, features, libm)", lambda f: core_src(f) or f == "builtins/gen/prelude.js" or f == "crates/djs-core/Cargo.toml"),
    ("cell layouts (heap/cells.rs: layout hash)", lambda f: f == "crates/djs-core/src/heap/cells.rs"),
    ("well-known atoms (heap/atom.rs)", lambda f: f == "crates/djs-core/src/heap/atom.rs"),
    ("opcode table (bytecode/ops.rs)", lambda f: f == "crates/djs-core/src/bytecode/ops.rs"),
    ("intrinsic table (bytecode/intrinsics.rs)", lambda f: f == "crates/djs-core/src/bytecode/intrinsics.rs"),
    ("realm slots (interp/realm.rs)", lambda f: f == "crates/djs-core/src/interp/realm.rs"),
    ("builtins in JS (builtins/gen/prelude.js)", lambda f: f == "builtins/gen/prelude.js"),
    ("compiler (compiler/)", lambda f: f.startswith("crates/djs-core/src/compiler/") and core_src(f)),
    ("object/shape code (heap/object.rs, shape.rs)", lambda f: f in ("crates/djs-core/src/heap/object.rs", "crates/djs-core/src/heap/shape.rs")),
    ("base image making (heap/compact.rs, vm/image.rs, djs-prelude)", lambda f: f in ("crates/djs-core/src/heap/compact.rs", "crates/djs-core/src/vm/image.rs") or f.startswith("crates/djs-prelude/")),
    ("collector (heap/gc.rs, compactor.rs)", lambda f: f in ("crates/djs-core/src/heap/gc.rs", "crates/djs-core/src/heap/compactor.rs")),
    ("pi adapter (js/pi/vm.ts)", lambda f: f == "js/pi/vm.ts"),
    ("pi's prelude copy (tests/pi/prelude-source.ts)", lambda f: f == "tests/pi/prelude-source.ts"),
    ("snapshot container (js/src/snapshot.ts)", lambda f: f == "js/src/snapshot.ts"),
    ("ABI (crates/djs-wasm/src/lib.rs)", lambda f: f.startswith("crates/djs-wasm/src/")),
])
keys = list(issues)
def table(sel, title):
    n = len(sel)
    print(f"\n{title}: {n} issues ({issues[sel[0]]['date']} .. {issues[sel[-1]]['date']})")
    for name, pred in inputs.items():
        hit = [k for k in sel if any(pred(f) for f in issues[k]["files"])]
        print(f"  {name}: {len(hit)} of {n} ({100*len(hit)/n:.0f}%)")
table(keys, "all issues on main")
table(keys[-30:], "the last 30 issues")
print("\nissue count", len(keys), "first", keys[0], "last", keys[-1])

# With two commits: which intrinsics keep their name but change their numeric id (the
# id is what an image's bytecode and builtin closures store).
if len(sys.argv) == 3:
    def names(rev):
        src = subprocess.run(["git", "show", f"{rev}:crates/djs-core/src/bytecode/intrinsics.rs"], capture_output=True, text=True).stdout
        return re.findall(r'\bi\("(\w+)"', src[src.index("pub const INTRINSICS"):])
    a, b = names(sys.argv[1]), names(sys.argv[2])
    moved = [n for n in a if n in b and a.index(n) != b.index(n)]
    print(f"\nintrinsics {sys.argv[1]} -> {sys.argv[2]}: {len(a)} -> {len(b)}; {len(moved)} keep their name and change their id")
    for n in ["FunctionPrototypeBehavior", "PromiseResolveFunction", "PromiseRejectFunction", "PromiseAllResolveElement"]:
        if n in a and n in b:
            print(f"  {n}: {a.index(n)} -> {b.index(n)} (id {a.index(n)} is {b[a.index(n)]} on the newer build)")
