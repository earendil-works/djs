#!/usr/bin/env python3
"""cpuprof.py <file.cpuprofile> [top]: self time by function (wasm functions by name)."""
import json, sys, collections
p = json.load(open(sys.argv[1])); top = int(sys.argv[2]) if len(sys.argv) > 2 else 30
nodes = {n["id"]: n for n in p["nodes"]}
self_t = collections.Counter()
dt = p["timeDeltas"]
for s, d in zip(p["samples"], dt):
    n = nodes[s]; cf = n["callFrame"]
    self_t[cf["functionName"] or cf["url"][-30:] or "(anon)"] += d
tot = sum(self_t.values())
for k, v in self_t.most_common(top):
    print(f"{100 * v / tot:6.1f}%  {k[:110]}")
