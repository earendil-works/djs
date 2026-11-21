#!/usr/bin/env python3
"""v8arm.py CODE.txt [OpName...]: reads V8's arm64 code of `run_hot` (the output of
`node --print-wasm-code-function-index=N`, which hotcode.sh makes; the last TurboFan
section counts) and prints what every bytecode pays:
- the frame size, the dispatch block (loop header to the table jump) and the back-edge
  block (the moves between the shared tail and the header);
- without names, one line per opcode: the length of the straight path from its
  jump-table target back to the loop header, with its calls, stack loads/stores and moves;
- with names (`Drop GetLoc0 Add`), those paths instruction by instruction.
The path follows unconditional branches and takes a conditional one only when it goes
back to the loop, so for arms with data-dependent branches (`lt`, `get_field`) it is one
static path, not the hot one: compare builds with it, measure with slope.py."""
import re, sys, os
path = sys.argv[1]
root = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../.."))
def op_names():
    """The opcodes in table order, from the `ops!` invocation of bytecode/ops.rs."""
    src = open(os.path.join(root, "crates/djs-core/src/bytecode/ops.rs")).read()
    body = src[src.index("\nops! {"):]
    body = body[:body.index("\n}\n")]
    return re.findall(r"^\s+([A-Z][A-Za-z0-9]*) [a-z_0-9]+\s*\(", body, re.M)
ops = op_names()
ins = {}      # off -> (mn, args)
order = []
tables = {}   # off -> target off
tf = False
for l in open(path, errors='replace'):
    if l.startswith('compiler:'):
        tf = 'TurboFan' in l or 'Turbo' in l
        ins.clear(); order.clear(); tables.clear()
    m = re.match(r'0x[0-9a-f]+\s+([0-9a-f]+)\s+jump table entry ([0-9a-f]+)', l)
    if m:
        tables[int(m.group(1), 16)] = int(m.group(2), 16); continue
    m = re.match(r'0x([0-9a-f]+)\s+([0-9a-f]+)\s+([0-9a-f]{8})\s+(\S+)\s*(.*)', l)
    if m:
        off = int(m.group(2), 16)
        ins[off] = (m.group(4), m.group(5).strip(), int(m.group(1), 16))
        order.append(off)
base_addr = ins[order[0]][2] - order[0]
def target(args):
    m = re.search(r'\(addr 0x([0-9a-f]+)\)', args)
    return int(m.group(1), 16) - base_addr if m else None
# the dispatch: the br x16 whose table has >= 200 entries
disp = None
for i, off in enumerate(order):
    if ins[off][0] == 'br':
        # find adr x16 before
        for j in range(i - 1, i - 8, -1):
            if ins[order[j]][0] == 'adr':
                t = target(ins[order[j]][1])
                n = 0
                while t + 4 * n in tables: n += 1
                if n >= 200:
                    disp = (off, t, n)
                break
    if disp: break
br_off, tab, ntab = disp
# loop header: walk back from br to the interrupt check (ldurb ... [x26, #-..]) 
hdr = None
i = order.index(br_off)
for j in range(i, i - 40, -1):
    mn, a, _ = ins[order[j]]
    if mn == 'ldurb' and 'x26' in a:
        hdr = order[j]; break
if hdr is None:
    # fall back: 20 instructions before
    hdr = order[i - 20]
# the back-edge block: instructions right before hdr that follow an unconditional b
k = order.index(hdr)
be = hdr
for j in range(k - 1, k - 30, -1):
    if ins[order[j]][0] in ('b', 'br', 'ret'):
        be = order[j + 1]; break
frame = None
for off in order[:12]:
    mn, a, _ = ins[off]
    if mn == 'sub' and a.startswith('sp, sp'):
        frame = a
print(f"code {4*len(order)} B ({len(order)} instr), frame: {frame}, dispatch br at {br_off:x}, header {hdr:x}, back-edge block {be:x}, table {ntab}")
def show(lo, hi):
    for off in order[order.index(lo):order.index(hi) + 1]:
        mn, a, _ = ins[off]
        print(f"   {off:5x}  {mn} {a}")
def trace(start, limit=400):
    """Follow from start until reaching [be, hdr]; returns (count, list)."""
    off = start; n = 0; out = []; seen = set()
    while n < limit:
        if be <= off <= hdr:
            # count the back-edge block instrs up to hdr
            while off < hdr:
                out.append((off, ins[off])); n += 1; off += 4
            return n, out, 'loop'
        if off in seen or off not in ins: return n, out, 'stuck'
        seen.add(off)
        mn, a, _ = ins[off]
        out.append((off, ins[off])); n += 1
        if mn == 'b':
            off = target(a); continue
        if mn in ('br', 'ret', 'brk', 'udf'):
            return n, out, mn
        if mn in ('bl', 'blr'):
            out[-1] = (off, (mn, a + '   <<< CALL', 0))
        if mn.startswith('b.') or mn in ('cbz', 'cbnz', 'tbz', 'tbnz'):
            t = target(a)
            if t is not None and be <= t <= hdr:
                off = t; continue
        off += 4
    return n, out, 'limit'


opname = sys.argv[2]
k = ops.index(opname)
start = tables[tab + 4 * k]
import sys
sys.setrecursionlimit(10000)
res = []
def dfs(off, n, path, seen, depth):
    while True:
        if be <= off <= hdr:
            m = n
            o = off
            while o < hdr: m += 1; o += 4
            res.append((m, list(path)))
            return
        if off in seen or off not in ins or n > 140: return
        seen = seen | {off}
        mn, a, _ = ins[off]
        n += 1
        if mn == 'b':
            off = target(a); continue
        if mn in ('br', 'ret', 'brk', 'udf', 'bl', 'blr'): return
        if mn.startswith('b.') or mn in ('cbz', 'cbnz', 'tbz', 'tbnz'):
            t = target(a)
            if t is not None and depth < 26:
                dfs(t, n, path + [(off, 'T')], seen, depth + 1)
                path = path + [(off, 'N')]
                depth += 1
        off += 4
dfs(start, 0, [], frozenset(), 0)
res.sort(key=lambda r: r[0])
print(opname, "call-free paths to the loop:", len(res), "lengths:", [r[0] for r in res[:24]])
want = int(sys.argv[3]) if len(sys.argv) > 3 else None
if want is not None:
    m, path = res[want]
    choice = dict(path)
    off = start; n = 0
    while not (be <= off <= hdr):
        mn, a, _ = ins[off]
        print(f"   {off:5x}  {mn} {re.sub(r' *.addr 0x[0-9a-f]+.', '', a)}")
        if mn == 'b': off = target(a); continue
        if off in choice and choice[off] == 'T': off = target(a); continue
        off += 4

sig = os.environ.get("SIG")
if sig:
    out = []
    for m, path in res:
        choice = dict(path); off = start; txt = []
        while not (be <= off <= hdr):
            mn, a, _ = ins[off]; txt.append(mn + " " + a)
            if mn == 'b': off = target(a); continue
            if off in choice and choice[off] == 'T': off = target(a); continue
            off += 4
        t = "\n".join(txt)
        if all((s[1:] not in t) if s.startswith("!") else (s in t) for s in sig.split(";")):
            out.append((m, sum(1 for x in txt if '[sp' in x)))
    print("sig", sig, "->", out[:12])
