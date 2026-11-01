#!/usr/bin/env bash
# libsym.sh <trace>: names the system-library functions in a trace (lldb image lookup)
tools="$(cd "$(dirname "$0")" && pwd)"; out="${DJS_PERF_DIR:-$tools/../../work/scratch/perf}"; cd "$out"
python3 - "$1" > /tmp/libaddrs.txt <<'PY'
import sys, collections
c = collections.Counter()
for l in open(sys.argv[1]):
    pc = int(l.split()[0], 16)
    if pc >= 0x180000000: c[pc] += 1
for pc, n in c.items(): print(f"{pc:x} {n}")
PY
{ echo "settings set target.disable-aslr true"; echo "b main"; echo "run";
  awk '{print "image lookup -a 0x"$1}' /tmp/libaddrs.txt; echo kill; echo quit; } > /tmp/libsym.lldb
lldb -b -s /tmp/libsym.lldb -- target/release/djs run --body "$tools/w/empty.js" 2>/dev/null | python3 -c '
import sys, re, collections
counts = {l.split()[0]: int(l.split()[1]) for l in open("/tmp/libaddrs.txt")}
cur = None; agg = collections.Counter()
for l in sys.stdin:
    m = re.search(r"image lookup -a 0x([0-9a-f]+)", l)
    if m: cur = m.group(1); continue
    m = re.search(r"Summary: (\S+)`(\S+)", l)
    if m and cur: agg[m.group(1) + "`" + m.group(2).split("+")[0]] += counts.get(cur, 0); cur = None
for k, v in agg.most_common(15): print(f"{v:8} {k}")
'
