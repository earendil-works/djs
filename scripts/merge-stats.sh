#!/usr/bin/env bash
# Summarizes the merge log (0079): scripts/merge-stats.sh [HOURS]  (default 24)
#
# From work/scratch/ of the primary checkout, written by merge-to-main.sh:
#   merge-log.tsv    one line per merge (lock waits, rebase, gate, total, outcome, merge
#                    lock held, attempts)
#   gate-steps.tsv   the gate's per-step wall and CPU seconds
#   slow-tests.tsv   the gate's 10 slowest unit tests and test262 tests per merge
# Prints merges, failures, lock waits and gate times per hour, the median time of each
# gate step, and the slowest tests in the window.
set -euo pipefail
common=$(git -C "$(dirname "$0")" rev-parse --path-format=absolute --git-common-dir)
exec python3 - "$(dirname "$common")/work/scratch" "${1:-24}" <<'PY'
import collections, datetime as dt, os, statistics as st, sys

scratch, hours = sys.argv[1], float(sys.argv[2])
since = dt.datetime.now() - dt.timedelta(hours=hours)

def rows(name, n):
    p = os.path.join(scratch, name)
    if not os.path.exists(p):
        return []
    out = []
    for line in open(p, encoding="utf-8"):
        f = line.rstrip("\n").split("\t")
        if len(f) < n:
            continue
        try:
            t = dt.datetime.strptime(f[0], "%Y-%m-%d %H:%M:%S")
        except ValueError:
            continue
        if t >= since:
            out.append([t] + f[1:])
    return out

def med(xs):
    return f"{st.median(xs):.0f}" if xs else "-"

merges = rows("merge-log.tsv", 7)
print(f"merges since {since:%F %H:%M} ({hours:g} h): {len(merges)} attempts, "
      f"{sum(1 for m in merges if m[6] == 'merged')} merged")
if merges:
    print("\nhour              tries merged failed(rebase/gate/ff)  lock wait med/max  gate med/max  total med")
    by = collections.defaultdict(list)
    for m in merges:
        by[m[0].strftime("%F %H:00")].append(m)
    for h in sorted(by):
        ms = by[h]
        fails = collections.Counter(m[6] for m in ms)
        wait = [int(m[2]) for m in ms]
        gate = [int(m[4]) for m in ms if m[6] in ("merged", "failed-gate", "failed-ff")]
        print(f"{h}  {len(ms):5} {fails['merged']:6}   "
              f"{fails['failed-rebase']:3}/{fails['failed-gate']}/{fails['failed-ff']}"
              f"{'':12}{med(wait):>5}/{max(wait):<5}     {med(gate):>4}/{max(gate) if gate else '-':<5}"
              f"  {med([int(m[5]) for m in ms]):>5}")
    # The merge lock's hold time: its own column since D2666 (the gate runs outside it),
    # rebase plus gate before.
    held = sum(int(m[7]) if len(m) > 7 else int(m[3]) + int(m[4]) for m in merges)
    span = (dt.datetime.now() - min(m[0] for m in merges)).total_seconds()
    print(f"\nlock held ~{100 * held / max(span, 1):.0f}% of the time since the first attempt")
    failed = sorted((m for m in merges if m[6] != "merged"), key=lambda m: m[0])
    if failed:
        print("failed attempts:")
        for m in failed[-10:]:
            print(f"  {m[0]:%F %H:%M}  {m[1]}  {m[6]}")

steps = rows("gate-steps.tsv", 5)
if steps:
    print("\ngate step (median over the window)        wall s   CPU s   runs")
    by = collections.defaultdict(list)
    order = []
    for s in steps:
        if s[2] not in by:
            order.append(s[2])
        by[s[2]].append((float(s[3] or 0), float(s[4] or 0)))
    for name in order:
        v = by[name]
        print(f"  {name[:40]:40} {st.median(w for w, _ in v):7.0f} {st.median(c for _, c in v):7.0f} {len(v):6}")

slow = rows("slow-tests.tsv", 5)
for kind in ("test", "test262"):
    v = collections.defaultdict(list)
    for s in slow:
        if s[2] == kind:
            v[s[4]].append((s[0], float(s[3])))
    if not v:
        continue
    print(f"\nslowest {'unit/integration tests' if kind == 'test' else 'test262 tests'} "
          "(max s, latest s, gates in the top 10, first seen):")
    for name, xs in sorted(v.items(), key=lambda kv: -max(x for _, x in kv[1]))[:10]:
        print(f"  {max(x for _, x in xs):7.2f} {xs[-1][1]:7.2f} {len(xs):4}  "
              f"{min(t for t, _ in xs):%m-%d %H:%M}  {name}")
PY
