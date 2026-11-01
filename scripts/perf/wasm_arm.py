#!/usr/bin/env python3
"""wasm_arm.py <func.dis> <br_table index> <entry>: prints the wasm code a br_table entry
of a function jumps to (from wasm-objdump -d output), following the structured control
flow: from the end of the targeted block up to the next branch/end, then (recursively,
once) the straight `br` continuations. Used to read fast_step's arms in run_loop."""
import sys, re
lines = open(sys.argv[1]).read().split("\n")
ins = []
for l in lines:
    m = re.match(r"\s*([0-9a-f]+):[^|]*\|\s*(.*)$", l)
    if m and m.group(2).strip():
        ins.append((int(m.group(1), 16), m.group(2).strip()))
# match block structure
stack = []; end_of = {}; kind = {}
for i, (a, t) in enumerate(ins):
    op = t.split()[0]
    if op in ("block", "loop", "if", "try"):
        stack.append(i); kind[i] = op
    elif op == "end" and stack:
        s = stack.pop(); end_of[s] = i
def target(i, depth):
    """Instruction index execution continues at for a branch at i to label depth."""
    st = []
    for j in range(i, -1, -1):
        pass
    # find enclosing blocks of i
    encl = [s for s, e in end_of.items() if s < i < e]
    encl.sort(reverse=True)  # innermost first
    b = encl[depth]
    return b + 1 if kind[b] == "loop" else end_of[b] + 1
tables = [i for i, (a, t) in enumerate(ins) if t.startswith("br_table")]
ti = tables[int(sys.argv[2])]
labels = [int(x) for x in ins[ti][1].split()[1:]]
entry = int(sys.argv[3])
pc = target(ti, labels[entry])
n = 0
seen = 0
while n < 200:
    a, t = ins[pc]
    print(f"  {a:06x}  {t}")
    n += 1
    op = t.split()[0]
    if op == "br":
        pc = target(pc, int(t.split()[1])); seen += 1
        print("        -> continue")
        if seen > 3: break
        continue
    if op in ("return", "unreachable", "br_table"):
        break
    pc += 1
print(f"{n} wasm instructions")
