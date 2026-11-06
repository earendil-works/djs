#!/usr/bin/env python3
"""The merge gate (scripts/gate.sh runs this; TESTING.md "The gate", D2660-D2669).

    scripts/gate.sh [--full] [--no-cache] [--only LEG,...] [--list] [--clear-cache]

A commit must pass the gate before main fast-forwards to it. The gate is a set of *legs*
(checks) and the *builds* they need. Builds sharing a cargo target directory run one at a
time (cargo locks it); everything else runs at once, at the machine's full width (D2661).

Each leg is cached by its inputs (D2662): the key hashes the leg's name, the committed tree
restricted to the leg's input class, and what the leg reads outside the repository (the
toolchain, the qjs binary, vendor/test262, pi-private). A leg whose key passed before is
not run again; a build whose legs are all cached is not built. `--full` reads no cache and
also runs the full-only legs (D2664).

The input classes (D2663), from widest to narrowest:
  all     every tracked file
  nodocs  all minus docs/, work/, .github/ and the top-level *.md files
  rust    nodocs minus js/ (but js/src/layout.generated.ts, which a djs-core test reads)
          and examples/
scripts/ is in every class, so a change to the gate reruns every leg. The `paths` leg
(always run) fails when Rust code starts naming a path outside the rust class, so the
classes stay true.
"""

import fcntl
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
os.chdir(ROOT)
NCPU = os.cpu_count() or 4
G = "target/gate"  # logs, build manifests, binary copies, per-leg times

# ---------------------------------------------------------------------------------------
# Input classes


def _common_root():
    common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
                            capture_output=True, text=True, check=True).stdout.strip()
    return Path(common).parent


PRIMARY = _common_root()
CACHE = Path(os.environ.get("DJS_GATE_CACHE", PRIMARY / "work/scratch/gate-cache"))
DOCS = re.compile(r"^(docs/|work/|\.github/|[^/]+\.md$)")
JS_ONLY = re.compile(r"^(js/|examples/)")
# Generated from Rust and checked by a Rust test: in the rust class.
RUST_KEEP = {"js/src/layout.generated.ts", "js/src/dump-runtime.js"}


def in_class(cls, path):
    if cls == "all":
        return True
    if DOCS.match(path):
        return False
    if cls == "nodocs":
        return True
    assert cls == "rust", cls
    return not JS_ONLY.match(path) or path in RUST_KEEP


_tree = None


def tree():
    """(path, blob sha) of every tracked file of HEAD."""
    global _tree
    if _tree is None:
        out = subprocess.run(["git", "ls-tree", "-r", "-z", "HEAD"], capture_output=True,
                             check=True).stdout.decode()
        _tree = []
        for entry in out.split("\0"):
            if entry:
                meta, path = entry.split("\t", 1)
                _tree.append((path, meta))
    return _tree


_class_hashes = {}


def class_hash(cls):
    if cls in _class_hashes:
        return _class_hashes[cls]
    h = hashlib.sha256(cls.encode())
    for path, meta in tree():
        if in_class(cls, path):
            h.update(f"{meta}\t{path}\n".encode())
    _class_hashes[cls] = h.hexdigest()
    return _class_hashes[cls]


# ---------------------------------------------------------------------------------------
# What legs read outside the repository (TESTING.md lists them)


def _run(*cmd, cwd=None):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, cwd=cwd).stdout
    except OSError:
        return "missing"


def _stat(path):
    try:
        st = Path(path).stat()
        return f"{path} {st.st_size} {st.st_mtime_ns}"
    except OSError:
        return f"{path} missing"


def _git_state(path):
    if not Path(path, ".git").exists():
        return f"{path} missing"
    head = _run("git", "rev-parse", "HEAD", cwd=path).strip()
    dirty = hashlib.sha256(_run("git", "status", "--porcelain", "--untracked-files=normal",
                                cwd=path).encode()).hexdigest()
    return f"{path} {head} {dirty}"


def _externals():
    pi = os.environ.get("PI_PRIVATE", str(Path.home() / "Development/pi-private"))
    return {
        # Every leg: the tools that build and run it, and the oracle and data files that
        # tests of every crate may read.
        "base": lambda: "\n".join([
            _run("rustc", "-vV"), _run("cargo", "-V"), _run("node", "--version"),
            sys.version, _run("uname", "-sm"),
            _stat(os.environ.get("DJS_QJS", Path.home() / "Development/dquickts/qjs")),
            _git_state(os.environ.get("DJS_TEST262", PRIMARY / "vendor/test262")),
            _stat(PRIMARY / "vendor/unicode/17.0.0/NormalizationTest.txt"),
        ]),
        # The npm package's leg: npm packs and installs it (TypeScript is pinned by
        # js/package-lock.json, in the tree).
        "npm": lambda: _run("npm", "--version"),
        # pi's codemode tests and prelude, and its installed packages.
        "pi": lambda: "\n".join([_git_state(pi), _stat(Path(pi) / "node_modules/.package-lock.json")]),
    }


EXTERNALS = _externals()
_ext_cache = {}


def external(name):
    if name not in _ext_cache:
        _ext_cache[name] = EXTERNALS[name]()
    return _ext_cache[name]


# ---------------------------------------------------------------------------------------
# Legs and builds


class Job:
    def __init__(self, name, cmd, *, needs=(), cls=None, group=None, ext=("base",),
                 full_only=False, summary=None, build=False):
        self.name, self.cmd, self.needs, self.cls = name, cmd, list(needs), cls
        self.group, self.ext, self.full_only, self.summary, self.build = group, ext, full_only, summary, build
        self.state = "pending"  # pending, skipped, cached, running, ok, failed
        self.wall = self.cpu = 0.0
        self.log = f"{G}/log/{re.sub(r'[^A-Za-z0-9_.-]+', '-', name).strip('-')}.log"

    def key(self):
        h = hashlib.sha256(f"leg {self.name}\n{class_hash(self.cls)}\n".encode())
        for e in self.ext:
            h.update(external(e).encode())
        return h.hexdigest()


def jobs():
    """The gate. A leg has an input class (`cls`); a build has none and runs only when a
    leg that needs it runs. `group` names the cargo target directory a job builds in:
    jobs of one group run one at a time."""
    tgt = "target"
    # Cargo's artifact messages, through `stream` (which starts each new executable once as
    # soon as it is linked, see cmd_stream) into a manifest the legs read.
    cargo_json = "--message-format=json-render-diagnostics | python3 scripts/gate.py stream"
    exe = f"python3 scripts/gate.py exe"
    tests = f"python3 scripts/gate.py tests"
    size_mod = "target/size/wasm32-unknown-unknown/wasm/djs_wasm.wasm"
    slim_mod = "target/size-slim/wasm32-unknown-unknown/wasm/djs_wasm.wasm"
    wasm = "target/wasm-dev/wasm32-unknown-unknown/debug/djs_wasm.wasm"
    nodump = "target/wasm-nodump/wasm32-unknown-unknown/debug/djs_wasm.wasm"
    J = Job
    return [
        # Cheap and always run: formatting, the registry against main (which is not in
        # the key), and the input classes' guard.
        J("fmt", "cargo fmt --all --check"),
        J("registry", "python3 scripts/registry.py check main"),
        J("paths", "python3 scripts/gate.py paths"),
        J("targets", "python3 scripts/gate.py targets"),
        # 0214: the merge drivers of derived files, through scripted three-way merges.
        J("merge drivers", "python3 scripts/merge-driver-test.py", cls="nodocs"),
        # 0231: the gate's recognition of incremental corruption in linker output.
        J("gate self-test", "python3 scripts/gate-test.py", cls="nodocs"),

        # --- Host builds. Each cargo configuration (package and features) has its own target
        # directory, so that they build at once (cargo locks a target directory's profile).
        J("build:tests", f"cargo test --workspace --no-run --quiet {cargo_json} > {G}/tests.json",
          group=tgt, build=True),
        J("build:core-tests", f"CARGO_TARGET_DIR=target/gate-core cargo test -p djs-core --lib --no-run --quiet"
          f" {cargo_json} > {G}/core-tests.json", group="target/gate-core", build=True),
        J("build:difftest", f"CARGO_TARGET_DIR=target/gate-difftest cargo build -p djs-difftest --bins --quiet"
          f" {cargo_json} > {G}/difftest.json && {exe} {G}/difftest.json djs-difftest djs-abidrive",
          group="target/gate-difftest", build=True),
        J("build:test262", f"CARGO_TARGET_DIR=target/gate-test262 cargo build -p djs-test262 --quiet"
          f" {cargo_json} > {G}/test262.json && {exe} {G}/test262.json djs-test262",
          group="target/gate-test262", build=True),
        J("build:docs", f"CARGO_TARGET_DIR=target/gate-docs cargo build -p djs-docs --quiet {cargo_json}"
          f" > {G}/docs.json && {exe} {G}/docs.json djs-docs", group="target/gate-docs", build=True),
        J("build:unsafe-tests", "CARGO_TARGET_DIR=target/gate-unsafe cargo test -p djs-core --features unsafe-heap"
          f" --no-run --quiet --test it --test e2e {cargo_json} > {G}/unsafe-tests.json",
          group="target/gate-unsafe", build=True),
        J("build:unsafe-release", "cargo build --release -p djs-difftest --features djs-core/unsafe-heap"
          f" --quiet {cargo_json} > {G}/unsafe-release.json"
          f" && {exe} {G}/unsafe-release.json djs-difftest", group="target/release", build=True),
        # builtins/gen's freshness is djs-bgen's `builtins_gen_is_fresh` test (in `test`), and
        # no crate has doctests (`targets` keeps it so): no programs of their own (D2790).

        # --- Clippy, host and wasm32 release (no debug_assertions, so warnings that only
        # appear there fail too; with djs-wasm's docs feature for the docs site's module).
        J("clippy", "CARGO_TARGET_DIR=target/clippy cargo clippy --workspace --all-targets --quiet -- -D warnings",
          cls="rust", group="target/clippy"),
        J("clippy (wasm32 release)", "CARGO_TARGET_DIR=target/clippy cargo clippy -p djs-core -p djs-wasm"
          " --features djs-wasm/docs --target wasm32-unknown-unknown --release --quiet -- -D warnings",
          cls="rust", group="target/clippy"),

        # --- Clippy on the other feature combinations that ship (D2668): djs-core natively
        # without `full`/`unicode` (slim), slim with the dumper (and `lazy`, as djs-wasm's
        # `dump` turns it on, D2990), full without them; and the
        # wasm modules in release as they ship: main and slim unchecked (D3100), dump, slim
        # with dump. Libraries only: the
        # tests build in the default combination (and slim's in its own legs).
        J("clippy (djs-core feature combinations)", "for f in '--no-default-features'"
          " '--no-default-features --features dump,lazy' '--no-default-features --features full,unicode';"
          " do CARGO_TARGET_DIR=target/clippy-combos cargo clippy -p djs-core $f --lib --quiet -- -D warnings"
          " || { echo \"in: cargo clippy -p djs-core $f\"; exit 1; }; done",
          cls="rust", group="target/clippy-combos"),
        *[J(f"clippy (wasm {name} module in release)",
            f"CARGO_TARGET_DIR=target/clippy-wasm-{name} cargo clippy -p djs-wasm --target wasm32-unknown-unknown"
            f" --release {flags} --quiet -- -D warnings", cls="rust", group=f"target/clippy-wasm-{name}")
          # One target directory each: four feature sets build nothing in common, and one
          # after another they were the gate's second-longest path (0197).
          for name, flags in (("main", "--features unsafe-heap"),
                              ("slim", "--no-default-features --features unsafe-heap"), ("dump", "--features dump"),
                              ("slim-dump", "--no-default-features --features dump"))],

        # --- wasm32 dev builds: djs-core no_std, and the modules the js tests load (0023),
        # with and without the dumper (0175, D2635).
        J("no_std/wasm build of djs-core",
          "CARGO_TARGET_DIR=target/wasm-dev cargo build -p djs-core --target wasm32-unknown-unknown --quiet",
          cls="rust", group="target/wasm-dev"),
        J("build:wasm", "CARGO_TARGET_DIR=target/wasm-dev cargo build -p djs-wasm --target wasm32-unknown-unknown"
          " --features compiled-prelude,dump --quiet", group="target/wasm-dev", build=True),
        J("build:wasm-nodump", "CARGO_TARGET_DIR=target/wasm-nodump cargo build -p djs-wasm"
          " --target wasm32-unknown-unknown --features compiled-prelude --quiet",
          group="target/wasm-nodump", build=True),

        # --- The shipped modules (D020, D2411): LTO, one codegen unit, one core each, and
        # unchecked arena access, as scripts/build-wasm.sh builds them (D3100).
        J("build:size", "CARGO_TARGET_DIR=target/size scripts/with-wasm-env.sh cargo build -p djs-wasm"
          " --target wasm32-unknown-unknown --profile wasm --features unsafe-heap --quiet",
          group="target/size", build=True),
        J("build:size-slim", "CARGO_TARGET_DIR=target/size-slim scripts/with-wasm-env.sh cargo build -p djs-wasm"
          " --target wasm32-unknown-unknown --profile wasm --no-default-features --features unsafe-heap --quiet",
          group="target/size-slim", build=True),

        # --- The slim build (D2411), natively in its own target directory.
        J("build:slim-tests", "CARGO_TARGET_DIR=target/slim cargo test -p djs-core --no-default-features --lib"
          f" --no-run --quiet {cargo_json} > {G}/slim-core-tests.json"
          " && CARGO_TARGET_DIR=target/slim cargo test -p djs-prelude --no-default-features"
          f" --features raw,registry --no-run --quiet {cargo_json} > {G}/slim-prelude-tests.json",
          group="target/slim", build=True),
        J("build:slim-bins", "CARGO_TARGET_DIR=target/slim cargo build -p djs-difftest --no-default-features"
          f" --quiet {cargo_json} > {G}/slim-difftest.json && {exe} {G}/slim-difftest.json djs-difftest"
          " && CARGO_TARGET_DIR=target/slim cargo build -p djs-test262 --no-default-features"
          f" --quiet {cargo_json} > {G}/slim-test262.json && {exe} {G}/slim-test262.json djs-test262",
          group="target/slim", build=True),

        # --- Unit and integration tests of the workspace (through scripts/test-runner.sh,
        # which records each test's time, 0079), djs-docs's apart: they read the guide.
        J("test", f"{tests} {G}/tests.json --not djs-docs --times", cls="rust", needs=["build:tests"],
          summary=r"^slowest tests:|^  +[0-9.]+s  "),
        J("test (djs-docs)", f"{tests} {G}/tests.json --only djs-docs", cls="all", needs=["build:tests"]),
        # 0103: the guide's generated regions are fresh, its examples print what they
        # printed, its links resolve and its writing rules hold (D2100, D3051).
        J("guide (docs/guide)", f"{G}/bin/djs-docs check", cls="all", needs=["build:docs"]),

        # --- The host-side JS package (0018), on the dev modules.
        J("js tests", f"DJS_WASM=$PWD/{wasm} DJS_WASM_NODUMP=$PWD/{nodump} node --test js/test/*.test.ts",
          cls="nodocs", needs=["build:wasm", "build:wasm-nodump"]),
        # 0229: the codemode workload set of the benchmarks page runs on djs and gives node's
        # results, with a dump and a load at every idle point (D3113); 0238: every payload of
        # the bridge benchmark reads back as it went in (D3165): about 4 s.
        J("codemode workloads", f"DJS_WASM=$PWD/{nodump} DJS_DUMP_WASM=$PWD/{wasm}"
          " node js/bench/engines/run.ts --quick --engines djs"
          " && node js/bench/engines/bridge.ts --quick --engines djs",
          cls="nodocs", needs=["build:wasm", "build:wasm-nodump"]),
        # 0228: the npm package stays publishable (D3109): it builds (tsc, with the
        # declarations), packs, and the packed tarball's smoke test runs in node. The gate's
        # builds stand in for the release ones: the main and slim modules as they ship, the
        # dev module for the dump module (the publish workflow tests the real ones).
        J("npm package", f'd=$(mktemp -d); t=$(scripts/npm-pack.sh --modules {size_mod} {slim_mod} {wasm}'
          ' --out "$d") && scripts/npm-test.sh "$t" node; rc=$?; rm -rf "$d"; exit $rc',
          cls="nodocs", ext=("base", "npm"), needs=["build:size", "build:size-slim", "build:wasm"]),
        # 0072: native and wasm builds leave byte-identical heap images and spend the same fuel.
        J("native vs wasm images", f"DJS_ABIDRIVE={G}/bin/djs-abidrive"
          f" ./scripts/determinism/native-vs-wasm.sh $PWD/{wasm}",
          cls="nodocs", needs=["build:wasm", "build:difftest"]),
        # 0018/0021: the wire format across languages (node encodes, djs decodes) and the
        # outbound reader on heap images of the same corpus plus hand-built cases.
        J("wire and heap reader cross-language",
          'w=$(mktemp -d) h=$(mktemp -d); node js/test/gen-cross.ts "$w" > /dev/null'
          f' && DJS_WIRE_CROSS="$w" {tests} {G}/core-tests.json -- wire_cross --ignored'
          f' && DJS_WIRE_CROSS="$w" DJS_READER_DIR="$h" {tests} {G}/core-tests.json -- reader_images --ignored'
          ' && node js/test/reader-cross.ts "$w" "$h"; rc=$?; rm -rf "$w" "$h"; exit $rc',
          cls="nodocs", needs=["build:core-tests"]),

        # --- The corpus (0016, 0072): ratchet, suspension and GC stress, heap images; and
        # with the @native builtins as bytecode (0063). (Dumps: the plain dumps leg.)
        # Lazy functions (D2990) here and in test262's leg; the other legs run eager.
        J("difftest", f"{G}/bin/djs-difftest --engine djs --lazy --ratchet --suspend-every 97 --gc-stress"
          f" --determinism --quiet --no-diff --times {G}/times/difftest.tsv",
          cls="rust", needs=["build:difftest"], summary=r"^(stress|ratchet):"),
        # 0201: plain dumps (D2830) at every idle point of the corpus (chains of at most 50),
        # continued in djs and in node; refusals only as tests/plain-refusals.txt lists.
        # 0207: and their size, against tests/scores/dump-size.tsv (D2901, per case: D2906), with the v2
        # programs' dumps (plain_dumps_at_every_idle_point writes them to DJS_PLAIN_OUT)
        # and the sleep-dump example's (the_sleep_example_dumps, DJS_SLEEP_OUT).
        J("plain dumps (djs and node)", f'd=$(mktemp -d); v=$(mktemp -d); s=$(mktemp -d); {G}/bin/djs-difftest'
          f' --engine djs --lazy --ratchet --quiet --no-diff --plain-out "$d" && node scripts/plain-node.mjs "$d"'
          ' --refusals tests/plain-refusals.txt'
          f' && DJS_PLAIN_OUT="$v" DJS_SLEEP_OUT="$s" {tests} {G}/core-tests.json --'
          ' plain_dumps_at_every_idle_point the_sleep_example_dumps'
          ' && node scripts/dump-size/dump-size.ts --score tests/scores/dump-size.tsv corpus="$d" v2="$v"'
          ' sleep="$s"; rc=$?; rm -rf "$d" "$v" "$s"; exit $rc',
          cls="rust", needs=["build:difftest", "build:core-tests"],
          summary=r"^(stress|ratchet|plain-node|dump-size):"),
        J("difftest (bytecode backend)", f"{G}/bin/djs-difftest --engine djs --backend bytecode --ratchet"
          f" --quiet --no-diff --times {G}/times/difftest-bytecode.tsv",
          cls="rust", needs=["build:difftest"], summary=r"^ratchet:"),
        # 0161: the unchecked accessor build (D2520): the verifier against corrupt images
        # and e2e in debug (the accessors assert their bounds), then the corpus in release,
        # where they really do not check, with suspension and GC stress.
        J("unsafe-heap tests", f"{tests} {G}/unsafe-tests.json", cls="rust", needs=["build:unsafe-tests"]),
        J("unsafe-heap difftest", f"{G}/bin/release/djs-difftest --engine djs --ratchet --suspend-every 97"
          f" --gc-stress --quiet --no-diff --times {G}/times/difftest-unsafe.tsv",
          cls="rust", needs=["build:unsafe-release"], summary=r"^(stress|ratchet):"),
        # 0007: test262's active group against the ratchet, with lazy functions (D2990) and,
        # alongside, eager as the shipped main module runs.
        J("test262", f"./scripts/fetch-test262.sh > /dev/null; e=$(mktemp); rc=0;"
          f" {{ {G}/bin/djs-test262 run --engine djs --group active --quiet"
          " --expected tests/test262/expected-failures.txt > \"$e\" 2>&1 && echo 0 >> \"$e\" || echo 1 >> \"$e\"; } &"
          f" {G}/bin/djs-test262 run --engine djs --lazy --group active --quiet --slowest 10"
          " --expected tests/test262/expected-failures.txt || rc=$?;"
          " wait; echo 'eager:'; sed '$d' \"$e\"; erc=$(tail -n 1 \"$e\"); rm -f \"$e\";"
          " [ \"$rc\" = 0 ] && [ \"$erc\" = 0 ]",
          cls="rust", needs=["build:test262"], summary=r"^ratchet:|^\| active|^slowest:|^ +[0-9.]+s |^eager:"),

        # --- Slim (D2411): the djs-core and djs-prelude tests that hold in both builds, the
        # codemode corpora with the stress modes, test262 without the families it leaves out
        # against its own ratchet.
        J("slim tests", f"{tests} {G}/slim-core-tests.json -- vm::slim_tests vm::frozen_tests vm::registry"
          f" && {tests} {G}/slim-prelude-tests.json", cls="rust", needs=["build:slim-tests"]),
        J("slim difftest", f"{G}/bin/slim/djs-difftest --engine djs --ratchet --suspend-every 97 --gc-stress"
          f" --determinism --quiet --no-diff --times {G}/times/difftest-slim.tsv codemode codemode-harvest",
          cls="rust", needs=["build:slim-bins"], summary=r"^(stress|ratchet):"),
        J("slim test262", f"./scripts/fetch-test262.sh > /dev/null && {G}/bin/slim/djs-test262 run --engine djs"
          " --group active --quiet --expected tests/test262/expected-failures-slim.txt",
          cls="rust", needs=["build:slim-bins"], summary=r"^ratchet:|^\| active"),

        # --- The shipped modules: the size budget (D020), pi's tests on the slim module
        # without pi's lockdown (D2411), and in the adapter's next mode (0176, D2650).
        J("size budget", f"python3 scripts/size/budget.py {size_mod} && python3 scripts/size/budget.py"
          f" {slim_mod} --slim", cls="rust", needs=["build:size", "build:size-slim"]),
        J("pi tests (slim without lockdown)", f"DJS_WASM=$PWD/{slim_mod} node js/pi/pi-tests.ts --no-lockdown"
          " --out target/pi-codemode-slim --expected tests/pi/slim-expected-failures.txt",
          cls="nodocs", ext=("base", "pi"), needs=["build:size-slim"], summary=r"^pi tests:"),
        J("pi tests (next mode)", f"DJS_WASM=$PWD/{size_mod} node js/pi/pi-tests.ts --next"
          " --out target/pi-codemode-next --expected tests/pi/expected-failures.txt",
          cls="nodocs", ext=("base", "pi"), needs=["build:size"], summary=r"^pi tests:"),

        # --- Full runs only (D2664): pi's tests in the next mode with a dump and a resume at
        # every idle point, on the release dump module (its own LTO build, 0176).
        J("pi tests (next mode with a dump at every idle point)", "scripts/pi-dump-every-idle.sh",
          cls="nodocs", ext=("base", "pi"), group="target/dump-wasm", full_only=True,
          summary=r"^pi tests:"),
    ]


# ---------------------------------------------------------------------------------------
# The cache: one file per passed key


def cache_get(key):
    try:
        return (CACHE / key[:2] / key).read_text().startswith("pass ")
    except (OSError, UnicodeDecodeError):
        return False  # missing, unreadable or corrupt: run the leg


def cache_put(key, job, sha):
    d = CACHE / key[:2]
    try:
        d.mkdir(parents=True, exist_ok=True)
        tmp = d / f".{key}.{os.getpid()}"
        tmp.write_text(f"pass {job.name}\t{sha}\t{time.strftime('%F %T')}\t{job.wall:.0f}s\n")
        tmp.replace(d / key)
    except OSError as e:
        print(f"gate: cannot write the cache entry of {job.name}: {e}", file=sys.stderr)


def cache_prune(max_age_days=14):
    """Entries unused for max_age_days go (a hit touches its entry)."""
    cutoff = time.time() - max_age_days * 86400
    for f in CACHE.glob("*/*"):
        try:
            if f.stat().st_mtime < cutoff:
                f.unlink()
        except OSError:
            pass


# ---------------------------------------------------------------------------------------
# Running


def run(argv):
    full = "--full" in argv
    use_cache = not full and "--no-cache" not in argv
    only = None
    if "--only" in argv:
        only = set(argv[argv.index("--only") + 1].split(","))
    all_jobs = jobs()
    by_name = {j.name: j for j in all_jobs}
    # `--only` takes a comma-separated list.
    assert not [j.name for j in all_jobs if "," in j.name], "a leg's name has a comma"
    if "--list" in argv:
        for j in all_jobs:
            kind = "build" if j.build else ("full-only" if j.full_only else (j.cls or "always"))
            print(f"{kind:10} {j.name}")
        return 0

    for d in ("log", "bin", "times"):
        Path(G, d).mkdir(parents=True, exist_ok=True)
    clean = subprocess.run(["git", "status", "--porcelain", "--untracked-files=normal"],
                           capture_output=True, text=True).stdout == ""
    if not clean and use_cache:
        print("gate: the worktree has uncommitted changes: no cache (the keys are the commit's)")
        use_cache = False
    sha = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    # One gate at a time per worktree: a second waits here rather than sharing target/.
    run_lock = open(Path(G, "run.lock"), "w")
    try:
        fcntl.flock(run_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        print("gate: another gate runs in this worktree; waiting for it", flush=True)
        fcntl.flock(run_lock, fcntl.LOCK_EX)

    def unchanged():
        """The worktree is still the commit the gate started on (nothing checked out
        another commit under it): only then may a pass be cached or reported."""
        head = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        return head == sha and (not clean or subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=no"],
            capture_output=True, text=True).stdout == "")

    # Which legs run: not cached, selected, full-only only in full runs.
    legs = [j for j in all_jobs if not j.build]
    for j in legs:
        if (j.full_only and not full) or (only and j.name not in only):
            j.state = "skipped"
        elif j.cls and use_cache and cache_get(j.key()):
            j.state = "cached"
            try:
                os.utime(CACHE / j.key()[:2] / j.key())
            except OSError:
                pass
    # Builds run when a running leg needs them (transitively).
    wanted = set()

    def want(j):
        for n in j.needs:
            if n not in wanted:
                wanted.add(n)
                want(by_name[n])
    for j in legs:
        if j.state == "pending":
            want(j)
    for j in all_jobs:
        if j.build and j.name not in wanted:
            j.state = "skipped"

    cached = [j.name for j in legs if j.state == "cached"]
    if cached:
        print(f"== cached ({len(cached)}): {', '.join(cached)}")
    todo = [j for j in all_jobs if j.state == "pending"]
    env = dict(os.environ, CARGO_BUILD_JOBS=str(NCPU), RUST_TEST_THREADS=str(NCPU),
               DJS_JOBS=str(NCPU), DJS_GATE="1")
    start = time.time()
    running = {}  # name -> Popen
    lock = threading.Condition()
    failed = []

    def launch(j):
        j.state = "running"
        j.t0 = time.time()
        log = open(j.log, "w")
        # The workspace's test build and its tests are the gate's longest path; the other
        # builds yield to them a little.
        prio = [] if not j.build or j.name == "build:tests" else ["nice", "-n", "4"]
        p = subprocess.Popen([*prio, "bash", "-o", "pipefail", "-ec", j.cmd], stdout=log, stderr=subprocess.STDOUT,
                             stdin=subprocess.DEVNULL, env=env, start_new_session=True)
        running[j.name] = p

        def waiter():
            _, status, ru = os.wait4(p.pid, 0)
            log.close()
            with lock:
                j.wall = time.time() - j.t0
                j.cpu = ru.ru_utime + ru.ru_stime
                j.state = "ok" if os.waitstatus_to_exitcode(status) == 0 else "failed"
                running.pop(j.name, None)
                p.returncode = 0  # reaped here
                lock.notify_all()
        threading.Thread(target=waiter, daemon=True).start()

    def ready(j):
        if j.state != "pending":
            return False
        if any(by_name[n].state != "ok" for n in j.needs):
            return False
        return not (j.group and any(by_name[r].group == j.group for r in running))

    def interrupted(signum, _frame):
        for p in list(running.values()):
            try:
                os.killpg(p.pid, signal.SIGTERM)
            except OSError:
                pass
        sys.exit(128 + signum)
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    signal.signal(signal.SIGHUP, interrupted)

    try:
        schedule(todo, by_name, ready, launch, running, lock, failed, clean, sha, unchanged)
    finally:
        for p in list(running.values()):
            try:
                os.killpg(p.pid, signal.SIGTERM)
            except OSError:
                pass

    wall = time.time() - start
    write_steps(all_jobs, wall)
    if not failed and not unchanged():
        print(f"gate: FAILED: the worktree is no longer at {sha[:10]} (another commit was"
              " checked out under the gate)")
        return 1
    if failed:
        print(f"gate: FAILED: {', '.join(j.name for j in failed)} ({wall:.0f}s)")
        return 1
    if full:
        record_full(sha)
    print(f"gate: OK ({wall:.0f}s; {len(cached)} legs cached)")
    if not full:
        remind_full()
    return 0


# rustc's incremental cache can go bad so that a link fails on symbols no object defines
# (D2896). Such a failure says nothing about the commit: the gate clears the incremental
# directories of the job's target directory, cleans djs-core and djs-prelude there (D3022: a
# corrupt rlib stays fingerprinted as fresh) and runs the job once more. In a pure-Rust
# workspace a real undefined symbol at link time essentially never happens, so any linker
# report of one counts (D3022's amendment, 0231): ld64's `Undefined symbols`, GNU ld's
# `undefined reference`, rust-lld's `undefined symbol:` (wasm), when the missing symbol is
# LLVM-local (`.llvm.<n>`, D2896, 0209) or the object referencing it is one of our crates'
# codegen units (`djs_core-<hash>.<cgu>.rcgu.o`; v0-mangled drop glue without the suffix
# went missing that way). The first such symbol can come many lines after the first
# undefined one (0205).
LINK_UNDEFINED = re.compile(r"Undefined symbols|undefined reference|undefined symbol:")
LLVM_LOCAL = re.compile(r"\.llvm\.[0-9]+\b")
OUR_OBJECT = re.compile(r"\bdjs_[a-z0-9_]+-[0-9a-f]+\.[A-Za-z0-9_.-]*\brcgu\.o\b")


def link_corruption(text):
    """Whether linker output `text` reports an undefined symbol of a bad incremental cache."""
    lines = text.splitlines()
    for i, line in enumerate(lines):
        if not LINK_UNDEFINED.search(line):
            continue
        window = lines[i:i + 400]
        if any(LLVM_LOCAL.search(w) for w in window):
            return True
        # The referencing object: on the line itself (rust-lld, GNU ld), on the line before
        # (GNU ld's "in function"), or under ld64's "referenced from:".
        if OUR_OBJECT.search(line) or (i > 0 and OUR_OBJECT.search(lines[i - 1])):
            return True
        for j, w in enumerate(window):
            if "referenced from:" in w and any(OUR_OBJECT.search(x) for x in window[j:j + 4]):
                return True
    return False


def incremental_corruption(log):
    try:
        text = Path(log).read_text(errors="replace")[-200000:]
    except OSError:
        return False
    return link_corruption(text)


def incremental_dirs(group, groups):
    """The incremental caches in target directory `group`: `incremental/` at its top and one
    or two levels down (profile, triple/profile), not inside another job's group or another
    target directory (marked by cargo's CACHEDIR.TAG)."""
    root = Path(group)
    others = [Path(g) for g in groups if g and Path(g) != root]
    out = []
    for d in [root / "incremental", *root.glob("*/incremental"), *root.glob("*/*/incremental")]:
        if not d.is_dir():
            continue
        inside = d.relative_to(root).parts[:-1]
        nested = any((root.joinpath(*inside[:k]) / "CACHEDIR.TAG").exists() for k in range(1, len(inside) + 1))
        if nested or any(o == d or o in d.parents for o in others if root in o.parents):
            continue
        out.append(d)
    return out


def clean_corrupt_rlibs(group):
    """`cargo clean -p djs-core -p djs-prelude` in target directory `group` (D3022): a crate
    built from a bad incremental cache stays fingerprinted as fresh, so clearing
    `incremental/` alone left the same link failing on every run (0215). The host's
    artifacts (build scripts, native jobs) and the wasm target's, where the group has
    them."""
    root = Path(group)
    if not root.is_dir():
        return
    base = ["cargo", "clean", "-q", "-p", "djs-core", "-p", "djs-prelude", "--target-dir", str(root)]
    # (A target triple's directory: `wasm32-unknown-unknown/`.)
    targets = [[]] + [["--target", t.name] for t in root.iterdir()
                      if t.is_dir() and t.name.count("-") >= 2]
    for extra in targets:
        subprocess.run(base + extra, cwd=ROOT, capture_output=True)


def schedule(todo, by_name, ready, launch, running, lock, failed, clean, sha, unchanged):
    """Launches ready jobs until all are done or one failed (then stops the others)."""
    with lock:
        while True:
            for j in todo:
                if ready(j) and not failed:
                    print(f"-> {j.name}", flush=True)
                    launch(j)
            done_now = [j for j in todo if j.state in ("ok", "failed") and not getattr(j, "reported", False)]
            again = False
            for j in done_now:
                if (j.state == "failed" and not failed and j.group and not getattr(j, "retried", False)
                        and not getattr(j, "stopped", False) and incremental_corruption(j.log)):
                    # Nothing else builds in the group's directory now (`ready`).
                    dirs = incremental_dirs(j.group, {x.group for x in by_name.values()})
                    for d in dirs:
                        shutil.rmtree(d, ignore_errors=True)
                    clean_corrupt_rlibs(j.group)
                    shutil.copyfile(j.log, j.log + ".corrupt")
                    print(f"!! {j.name}: rustc incremental corruption (link: undefined symbols in our crates);"
                          f" cleared {', '.join(map(str, dirs)) or 'nothing'}, running it again"
                          f" (the failed run's log: {j.log}.corrupt)", flush=True)
                    j.retried, j.state = True, "pending"
                    again = True
                    continue
                j.reported = True
                if getattr(j, "stopped", False):
                    print(f"-- {j.name}: stopped")
                    continue
                report(j)
                if j.state == "failed":
                    failed.append(j)
                elif j.cls and clean and unchanged():
                    cache_put(j.key(), j, sha)
            if again and not failed:
                continue  # launch the retried job
            if failed:
                # Fail fast: the first failure stops the rest.
                for name, p in list(running.items()):
                    by_name[name].stopped = True
                    try:
                        os.killpg(p.pid, signal.SIGTERM)
                    except OSError:
                        pass
                if not running:
                    break
            elif not running:
                if any(j.state == "pending" for j in todo):
                    stuck = [j.name for j in todo if j.state == "pending"]
                    print(f"gate: cannot schedule {stuck}", file=sys.stderr)
                    failed.append(by_name[stuck[0]])
                break
            lock.wait(1.0)



def report(j):
    if j.state == "failed":
        print(f"!! {j.name}: FAILED after {j.wall:.0f}s; its output ({j.log}):")
        try:
            sys.stdout.write(Path(j.log).read_text(errors="replace")[-60000:])
        except OSError:
            pass
        print(f"!! end of {j.name}", flush=True)
        return
    print(f"ok {j.name} ({j.wall:.0f}s)", flush=True)
    if j.summary:
        try:
            for line in Path(j.log).read_text(errors="replace").splitlines():
                if re.search(j.summary, line):
                    print(f"   {line}")
        except OSError:
            pass


def write_steps(all_jobs, wall):
    """target/gate-steps.tsv (0079): name, wall and CPU seconds of each job that ran, cached
    legs with 0 0 and a `(cached)` suffix, and a total; merge-to-main collects them."""
    lines = []
    for j in all_jobs:
        if j.state in ("ok", "failed"):
            lines.append(f"{j.name}\t{j.wall:.0f}\t{j.cpu:.1f}")
        elif j.state == "cached":
            lines.append(f"{j.name} (cached)\t0\t0")
    cpu = sum(j.cpu for j in all_jobs)
    lines.append(f"total\t{wall:.0f}\t{cpu:.1f}")
    Path("target/gate-steps.tsv").write_text("\n".join(lines) + "\n")
    # The slowest unit tests and test262 tests, for merge-to-main's slow-tests.tsv: from
    # this run's legs only (empty when they were cached).
    ran = {j.name for j in all_jobs if j.state == "ok"}
    for name, out in (("test", "target/gate-slowest-tests.tsv"), ("test262", "target/gate-slowest-test262.txt")):
        if name not in ran:
            Path(out).write_text("")
    times = Path(G, "test-times.tsv")
    if "test" in ran and times.exists():
        rows = sorted((l.split("\t") for l in times.read_text().splitlines() if l.count("\t") == 2),
                      key=lambda r: -float(r[0]))[:10]
        Path("target/gate-slowest-tests.tsv").write_text("".join("\t".join(r) + "\n" for r in rows))
    log = Path(by_name_log("test262"))
    if "test262" in ran and log.exists():
        text = log.read_text(errors="replace")
        part = text.split("slowest:", 1)[1].split("\n\n", 1)[0] if "slowest:" in text else ""
        Path("target/gate-slowest-test262.txt").write_text(
            "".join(l + "\n" for l in part.splitlines() if re.match(r"^ +[0-9]", l)))


def by_name_log(name):
    return Job(name, "").log


def remind_full():
    """The last full pass is older than a day: say so (it does not fail the gate)."""
    try:
        last = int((CACHE / "full-passes.tsv").read_text().splitlines()[-1].split("\t")[0])
    except (OSError, ValueError, IndexError):
        last = 0
    hours = (time.time() - last) / 3600
    if hours > 24:
        when = f"{hours:.0f} hours ago" if last else "never"
        print(f"gate: the last full run (scripts/gate.sh --full) passed {when}; one is due (TESTING.md \"Full runs\")")


def record_full(sha):
    """A full pass, for merge-to-main's reminder (D2664)."""
    f = CACHE / "full-passes.tsv"
    try:
        CACHE.mkdir(parents=True, exist_ok=True)
        with open(f, "a") as out:
            out.write(f"{int(time.time())}\t{time.strftime('%F %T')}\t{sha}\n")
    except OSError:
        pass


# ---------------------------------------------------------------------------------------
# Helpers the legs call


def artifacts(manifest):
    for line in Path(manifest).read_text().splitlines():
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") == "compiler-artifact" and m.get("executable"):
            yield m


def cmd_exe(args):
    """exe MANIFEST NAME...: link the named binaries a build produced into target/gate/bin
    (bin/release, bin/slim by the manifest's name), so that later builds that write the same
    path (another feature set) do not change what a leg runs."""
    manifest, names = args[0], args[1:]
    sub = {"unsafe-release.json": "release", "slim-difftest.json": "slim", "slim-test262.json": "slim"}
    dest = Path(G, "bin", sub.get(Path(manifest).name, ""))
    dest.mkdir(parents=True, exist_ok=True)
    found = set()
    for m in artifacts(manifest):
        if m["target"]["name"] in names and "bin" in m["target"]["kind"] and not m["profile"]["test"]:
            # A hard link, not a copy: macOS scans every new executable file on its first
            # run (syspolicyd), and a copy is a new file. Cargo replaces the linked file
            # rather than rewriting it, so a later build cannot change what the link runs.
            target = dest / m["target"]["name"]
            target.unlink(missing_ok=True)
            os.link(os.path.realpath(m["executable"]), target)
            found.add(m["target"]["name"])
    missing = set(names) - found
    if missing:
        print(f"gate exe: {manifest} built no {sorted(missing)}", file=sys.stderr)
        return 1
    return 0


def cmd_stream(_args):
    """stream: copy cargo's JSON messages from stdin to stdout and run each executable a
    build links once, with `--list` (test binaries) or `--help`, output discarded. macOS
    scans every new executable on its first run, one at a time (syspolicyd); doing that
    while the build goes on keeps it off the legs' path."""
    threads = []
    sem = threading.Semaphore(4)

    def warm(exe, is_test):
        with sem:
            t0 = time.time()
            try:
                subprocess.run([exe, "--list" if is_test else "--help"], stdin=subprocess.DEVNULL,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=120)
            except (OSError, subprocess.SubprocessError):
                pass
            print(f"gate stream: {Path(exe).name} first run {time.time() - t0:.1f}s", file=sys.stderr)
    for line in sys.stdin:
        sys.stdout.write(line)
        sys.stdout.flush()
        try:
            m = json.loads(line)
        except ValueError:
            continue
        if m.get("reason") != "compiler-artifact" or not m.get("executable") or m.get("fresh"):
            continue
        # Test binaries list their tests; of the programs only those known to just print
        # their usage on --help (djs-bgen without arguments would rewrite files).
        if m["profile"]["test"] or m["target"]["name"] in ("djs", "djs-difftest"):
            t = threading.Thread(target=warm, args=(m["executable"], m["profile"]["test"]), daemon=True)
            t.start()
            threads.append(t)
    # The build is done: legs may start. Runs still waiting for their check go on (or are
    # dropped, if not started yet: the leg's own run waits for the check then), so the
    # checks overlap the legs instead of delaying them. Measured on a core change: 77/85 s
    # this way, 88/100 s waiting for all of them, 105/105 s not running them (D2792).
    print(f"gate stream: cargo done; {len(threads)} new executables,"
          f" {sum(t.is_alive() for t in threads)} still waiting for their first run", file=sys.stderr)
    return 0


def cmd_tests(args):
    """tests MANIFEST [--only PKG | --not PKG] [--times] [-- ARGS...]: run the test binaries
    of a `cargo test --no-run` build in parallel, the way `cargo test` runs them one after
    another: in the package's directory, with its CARGO_MANIFEST_DIR and CARGO_PKG_NAME,
    through scripts/test-runner.sh."""
    extra = []
    if "--" in args:
        extra = args[args.index("--") + 1:]
        args = args[:args.index("--")]
    manifest = args[0]
    only = args[args.index("--only") + 1] if "--only" in args else None
    skip = args[args.index("--not") + 1] if "--not" in args else None
    times = Path(G, "test-times.tsv") if "--times" in args else None
    runs = []
    seen = set()
    for m in artifacts(manifest):
        if not m["profile"]["test"]:
            continue
        pdir = str(Path(m["manifest_path"]).parent)
        with open(m["manifest_path"], "rb") as f:
            name = tomllib.load(f)["package"]["name"]
        if (only and name != only) or (skip and name == skip) or m["executable"] in seen:
            continue
        seen.add(m["executable"])
        runs.append((name, pdir, m["executable"]))
    if not runs:
        print(f"gate tests: no test binaries in {manifest}", file=sys.stderr)
        return 1
    if times:
        times.write_text("")
    # The biggest binaries (djs-core's) first; each runs its tests on all cores.
    runs.sort(key=lambda r: -Path(r[2]).stat().st_size)
    results = {}
    sem = threading.Semaphore(max(2, NCPU // 3))

    def one(r):
        name, pdir, exe = r
        with sem:
            env = dict(os.environ, CARGO_MANIFEST_DIR=pdir, CARGO_PKG_NAME=name)
            if times:
                env["DJS_TEST_TIMES"] = str(times.resolve())
            t0 = time.time()
            p = subprocess.run([str(ROOT / "scripts/test-runner.sh"), exe, *extra], cwd=pdir, env=env,
                               capture_output=True, text=True)
            results[exe] = (p.returncode, p.stdout + p.stderr, time.time() - t0)
    threads = [threading.Thread(target=one, args=(r,)) for r in runs]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    rc = 0
    print("binaries: " + ", ".join(f"{Path(e).name} {results[e][2]:.1f}s"
                                   for e in sorted(results, key=lambda e: -results[e][2])[:8]))
    for name, _, exe in runs:
        code, out, _ = results[exe]
        if code != 0:
            rc = 1
            print(f"--- {name}: {Path(exe).name} failed ({code}):\n{out}")
    passed = sum(1 for c, _, _ in results.values() if c == 0)
    print(f"tests: {passed}/{len(runs)} binaries passed")
    if times and times.exists():
        rows = sorted((l.split("\t") for l in times.read_text().splitlines() if l.count("\t") == 2),
                      key=lambda r: -float(r[0]))[:10]
        print("slowest tests:")
        for secs, binary, test in rows:
            print(f"  {float(secs):7.2f}s  {binary}  {test}")
    return rc


# Rust code may name paths outside the rust class only here: each entry is a file and the
# path it names, with the reason it is safe (D2663).
PATHS_ALLOWED = {
    # The benchmark harness runs js/bench; no test runs it.
    ("crates/djs-bench/src/wasm.rs", "js/bench/wasm.bench.ts"),
    # In the rust class (RUST_KEEP).
    ("crates/djs-core/src/heap/layout.rs", "js/src/layout.generated.ts"),
    ("crates/djs-core/src/vm/dump_runtime.rs", "js/src/dump-runtime.js"),
}
# A repository path in a string literal: at its start, or after `../` (relative to a crate).
PATH_LITERAL = re.compile(r'(?:"|\.\./)((?:js|examples|docs|work|\.github)/[^"\s)]*|[A-Z][A-Za-z_-]*\.md(?="))')
DOC_LITERAL = re.compile(r"^docs/[a-z0-9-]+\.md$")  # help texts cite design docs


def cmd_paths(_args):
    """paths: Rust code outside djs-docs (whose legs read everything) must not name paths
    outside the rust class in string literals, or the cache would skip its tests when those
    paths change (D2663)."""
    bad = []
    for path, _ in tree():
        if not path.startswith("crates/") or path.startswith("crates/djs-docs/") or not path.endswith(".rs"):
            continue
        try:
            text = Path(path).read_text()
        except (OSError, UnicodeDecodeError):
            continue
        for n, line in enumerate(text.splitlines(), 1):
            if line.lstrip().startswith("//"):
                continue
            for m in PATH_LITERAL.finditer(line):
                named = m.group(1).rstrip(".,;:")
                if DOC_LITERAL.match(named) or (path, named) in PATHS_ALLOWED:
                    continue
                if not in_class("rust", named) or named.startswith(("js/", "examples/")):
                    if named not in RUST_KEEP:
                        bad.append(f"{path}:{n}: names {named}")
    if bad:
        print("gate paths: Rust code names files outside the rust input class (scripts/gate.py):")
        print("\n".join(f"  {b}" for b in bad))
        print("Move the file under tests/ or scripts/, or widen the class in scripts/gate.py (D2663).")
        return 1
    print("paths: OK")
    return 0


# rustdoc's code block attributes: a block whose info string has only these (or none) is
# Rust, and compiled as a doctest unless it is `ignore`d.
RUSTDOC_ATTRS = re.compile(r"^(rust|should_panic|no_run|compile_fail|test_harness|standalone_crate"
                           r"|edition\d+|ignore(-.*)?|E\d+)?$")
DOC_LINE = re.compile(r"^\s*//[/!] ?(.*)$")


def doctests_in(src_dir):
    """`file:line` of every doctest rustdoc would compile in a crate's sources."""
    found = []
    for f in sorted(Path(src_dir).rglob("*.rs")):
        fence = None
        for n, line in enumerate(f.read_text(errors="replace").splitlines(), 1):
            m = DOC_LINE.match(line)
            text = m.group(1).strip() if m else None
            if text is None:
                fence = None
                continue
            if fence is None:
                if text.startswith(("```", "~~~")):
                    fence = text[:3]
                    info = [t for t in re.split(r"[\s,]+", text[3:].strip()) if t]
                    rust = all(RUSTDOC_ATTRS.match(t) for t in info)
                    if rust and not any(t.startswith("ignore") for t in info):
                        found.append(f"{f}:{n}")
            elif text.startswith(fence):
                fence = None
    return found


def cmd_targets(_args):
    """targets: what the manifests leave out of `cargo test` must have nothing to test
    (D2790): a library with `doctest = false` has no doctests, a program with `test =
    false` no unit tests (no #[test], no cfg(test), no modules of its own)."""
    bad = []
    for manifest in sorted(Path("crates").glob("*/Cargo.toml")):
        crate = manifest.parent
        with open(manifest, "rb") as f:
            m = tomllib.load(f)
        lib = m.get("lib", {})
        if (crate / "src/lib.rs").exists() and lib.get("doctest", True) is False:
            bad += [f"{d}: a doctest, but {manifest} has doctest = false" for d in doctests_in(crate / "src")]
        for b in m.get("bin", []):
            if b.get("test", True) is not False:
                continue
            path = crate / b.get("path", f"src/bin/{b['name']}.rs")
            text = path.read_text()
            if re.search(r"#\[test\]|cfg\(test\)|^\s*(pub(\(\w+\))? )?mod \w+;", text, re.M):
                bad.append(f"{path}: tests or modules, but its [[bin]] has test = false")
    if bad:
        print("gate targets: cargo test would skip these (scripts/gate.py, D2790):")
        print("\n".join(f"  {b}" for b in bad))
        print("Remove the `doctest = false` / `test = false` line from the manifest.")
        return 1
    print("targets: OK")
    return 0


def main(argv):
    if argv[:1] == ["exe"]:
        return cmd_exe(argv[1:])
    if argv[:1] == ["stream"]:
        return cmd_stream(argv[1:])
    if argv[:1] == ["tests"]:
        return cmd_tests(argv[1:])
    if argv[:1] == ["targets"]:
        return cmd_targets(argv[1:])
    if argv[:1] == ["paths"]:
        return cmd_paths(argv[1:])
    if "--full" in argv and Path(".").resolve() == PRIMARY.resolve():
        # merge-to-main fast-forwards main in the primary checkout, which would move the
        # tree under a long run: full runs go in their own detached worktree (D2664).
        full_wt = PRIMARY / ".worktrees/full"
        if not (full_wt / ".git").exists():
            subprocess.run(["git", "worktree", "add", "-q", "--detach", str(full_wt), "main"], check=True)
        subprocess.run(["git", "-C", str(full_wt), "checkout", "-q", "--detach", "--force", "main"], check=True)
        subprocess.run(["git", "-C", str(full_wt), "clean", "-fdq"], check=True)
        print(f"gate: a full run of main ({subprocess.run(['git', 'rev-parse', '--short', 'main'], capture_output=True, text=True).stdout.strip()}) in {full_wt}", flush=True)
        os.execvp("bash", ["bash", str(full_wt / "scripts/gate.sh"), *argv])
    if "--clear-cache" in argv:
        shutil.rmtree(CACHE, ignore_errors=True)
        print(f"gate: cleared {CACHE}")
        return 0
    cache_prune()
    return run(argv)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
