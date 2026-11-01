import subprocess, sys
W="../../target/wasm32-unknown-unknown/wasm/djs_wasm.wasm"
cons = {
 "parens": lambda n: "(" * n + "1" + ")" * n,
 "arrays": lambda n: "[" * n + "]" * n,
 "objects": lambda n: "{a:" * n + "1" + "}" * n,
 "arrows": lambda n: "() => " * n + "1",
 "ternary": lambda n: "1 ? " * n + "1" + " : 0" * n,
 "unary": lambda n: "-" * n + "1",
 "calls": lambda n: "f(" * n + "1" + ")" * n,
 "funcs": lambda n: "function g(){ return " * n + "1" + "}" * n,
 "blocks": lambda n: "(() => {" + "{" * n + "}" * n + "})()",
 "ifs": lambda n: "(() => {" + "if (1) " * n + "; })()",
 "template": lambda n: "`${" * n + "1" + "}`" * n,
 "regexp_groups": lambda n: "/" + "(" * n + "a" + ")" * n + "/",
}
def ok(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    return r.returncode == 0, (r.stdout + r.stderr)[-200:]
for name, mk in cons.items():
    lo, hi = 1, 4000
    def src(n): return "const f = (x) => x; const v = " + mk(n) + ";\nawait tool(0); return 1;\n"
    while lo < hi:
        mid = (lo + hi + 1) // 2
        open("/tmp/deep.js", "w").write(src(mid))
        good, _ = ok(["../../target/debug/djs-abidrive", "/tmp/deep.js", "1000000"])
        if good: lo = mid
        else: hi = mid - 1
    open("/tmp/deep.js", "w").write(src(lo))
    wgood, wout = ok(["node", "abidrive.mjs", W, "/tmp/deep.js", "1000000"])
    open("/tmp/deep.js", "w").write(src(lo + 1))
    _, nout = ok(["../../target/debug/djs-abidrive", "/tmp/deep.js", "1000000"])
    print(f"{name:14} native max {lo:5}  wasm at that depth: {'ok' if wgood else 'FAIL ' + wout.strip().splitlines()[-1][:120]} | native at +1: {nout.strip().splitlines()[-3][:80] if nout.strip() else ''}")
