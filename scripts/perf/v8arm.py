#!/usr/bin/env python3
"""v8arm.py CODE.txt [OpName...]: reads V8's arm64 code of `run_hot` (the output of
`node --print-wasm-code-function-index=N`, which hotcode.sh makes; the last TurboFan
section counts) and prints what every bytecode pays:
- the frame size, the dispatch block (loop header to the table jump) and the back-edge
  block (the moves between the shared tail and the header);
- without names, one line per opcode: the length of the straight path from its
  jump-table target back to the loop header, with its calls, stack loads/stores and moves;
- with names (`Drop GetLoc0 Add`), those paths instruction by instruction.
- with `--paths OpName [SIG]`, the lengths (and stack loads and stores) of every
  call-free path of that arm back to the loop header, shortest first, over all outcomes
  of its conditional branches; SIG keeps the paths whose text has each `;`-separated
  part (`!part`: lacks it), e.g. `'#0x900000000;!#0x800000000'` for `get_field`'s hits
  on an object receiver. This is how to see what a data-dependent arm's hit costs.
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
names = sys.argv[2:]
if names and names[0] == '--paths':
    sys.setrecursionlimit(10000)
    start = tables[tab + 4 * ops.index(names[1])]
    sig = names[2].split(';') if len(names) > 2 else []
    res = []
    def walk(off, n, text, sp, seen, depth):
        while True:
            if be <= off <= hdr:
                n += (hdr - off) // 4
                t = "\n".join(text)
                if all((p[1:] not in t) if p.startswith('!') else (p in t) for p in sig):
                    res.append((n, sp))
                return
            if off in seen or off not in ins or n > 160: return
            seen = seen | {off}
            mn, a, _ = ins[off]
            n += 1; text = text + [mn + " " + a]; sp += '[sp' in a
            if mn == 'b':
                off = target(a); continue
            if mn in ('br', 'ret', 'brk', 'udf', 'bl', 'blr'): return
            if mn.startswith('b.') or mn in ('cbz', 'cbnz', 'tbz', 'tbnz'):
                t = target(a)
                if t is not None and depth < 26:
                    walk(t, n, text, sp, seen, depth + 1)
                    depth += 1
            off += 4
    walk(start, 0, [], 0, frozenset(), 0)
    res.sort()
    print(f"{names[1]}: {len(res)} call-free paths to the loop; (instructions, stack loads and stores):")
    print("  " + " ".join(f"({n}, {sp})" for n, sp in res[:16]))
    sys.exit(0)
hn = (br_off - hdr) // 4 + 1
if not names:
    print(f"dispatch (header..br): {hn} instr")
    show(hdr, br_off)
    print("back-edge block:")
    if be < hdr: show(be, hdr - 4)
    rows = []
    for k in range(ntab):
        n, out, why = trace(tables[tab + 4 * k])
        calls = sum(1 for _, (mn, a, _) in out if mn in ('bl', 'blr'))
        lds = sum(1 for _, (mn, a, _) in out if mn.startswith('ld') and '[sp' in a)
        sts = sum(1 for _, (mn, a, _) in out if mn.startswith('st') and '[sp' in a)
        mv = sum(1 for _, (mn, a, _) in out if mn == 'mov')
        rows.append((ops[k], n, why, calls, lds, sts, mv))
    for r in rows:
        print(f"{r[0]:22s} {r[1]:4d} {r[2]:6s} calls {r[3]} spload {r[4]} spstore {r[5]} mov {r[6]}")
else:
    for name in names:
        k = ops.index(name)
        n, out, why = trace(tables[tab + 4 * k])
        print(f"--- {name} ({k}): {n} instr to header ({why}), + dispatch {hn}")
        for off, (mn, a, _) in out:
            print(f"   {off:5x}  {mn} {a}")
