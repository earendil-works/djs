#!/usr/bin/env python3
"""Bytes of padded LEB128 immediates in a wasm code section (0052).

wasm-ld writes relocated immediates (call targets, memory addresses, the stack pointer
global, table indices) as fixed 5-byte LEBs so that it can patch them in place. Without
wasm-opt or `--compress-relocations` they stay padded. This counts what minimal LEBs
would save, from `wasm-objdump -d` output.

usage: wasm-objdump -d module.wasm | scripts/wasm-leb-padding.py
"""
import re
import sys
from collections import Counter


def minimal_len(value, signed):
    n = 1
    while True:
        if signed:
            if -(1 << (7 * n - 1)) <= value < (1 << (7 * n - 1)):
                return n
        elif value < (1 << (7 * n)):
            return n
        n += 1


waste, sites = Counter(), Counter()
for line in sys.stdin:
    m = re.match(r"\s*[0-9a-f]+: ((?:[0-9a-f]{2} )+)\s*\|\s*(\S+)\s*(-?\d+)?", line)
    if not m or not m.group(3):
        continue
    raw = bytes(int(x, 16) for x in m.group(1).split())
    op = m.group(2)
    if op not in ("call", "i32.const", "global.get", "global.set", "call_indirect"):
        continue
    # opcode byte (one for these), then the LEB immediate
    imm = raw[1:]
    if op == "call_indirect":
        # type index then table index; only the type index can be padded
        k = 0
        while imm[k] & 0x80:
            k += 1
        imm = imm[:k + 1]
    value = int(m.group(3))
    used = len(imm)
    need = minimal_len(value, op == "i32.const")
    if used > need:
        waste[op] += used - need
        sites[op] += 1
total = sum(waste.values())
for op in waste:
    print(f"{op:>14}: {sites[op]:>6} padded immediates, {waste[op]:>8,} bytes")
print(f"{'total':>14}: {sum(sites.values()):>6} padded immediates, {total:>8,} bytes")
