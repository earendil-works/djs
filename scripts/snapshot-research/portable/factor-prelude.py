#!/usr/bin/env python3
"""Splits pi's prelude (the evaluated PRELUDE_SOURCE) into two stages (issue 0142):

    (function (bridge) { <lockdown, helpers, output, store/load, console, text, ...>
                         return function configure(toolsJson, globalsJson, storeJson) {
                             <tools, ALL_TOOLS, declared globals, the store snapshot>; return api } })

Stage 1 depends on nothing per run, so its sealed image can be the snapshot reference
for every session of a prelude version; stage 2 runs per session. A scratch transform of
pi's text for measurements, not a proposal for pi's exact code: the order in which the
globals are defined changes (tools, ALL_TOOLS and the declared globals come last).

usage: factor-prelude.py pi-prelude.js > pi-prelude-factored.js
"""
import sys

s = open(sys.argv[1]).read()


def cut(text, start, end):
    a = text.index(start)
    b = text.index(end, a)
    return text[:a] + text[b:], text[a:b]


s = s.replace("(function (bridge, toolsJson, globalsJson, storeJson) {", "(function (bridge) {", 1)
s, block_a = cut(s, "\tconst tools = Object.create(null);", "\t// key -> JSON text.")
block_a = block_a.replace("\tconst allTools = [];", "\tallTools = [];", 1)
block_a = block_a.replace("\tconst toolsProxy = guard(", "\ttoolsProxy = guard(", 1)
old_b = (
    "\tconst stored = new Map(Object.entries(parse(storeJson)));\n"
    "\tconst writes = new Map();\n"
    "\tlet storedChars = 0;\n"
    "\tfor (const [key, json] of stored) storedChars += key.length + json.length;\n"
)
assert old_b in s
s = s.replace(
    old_b,
    "\tlet stored = new Map();\n\tconst writes = new Map();\n\tlet storedChars = 0;\n\tlet allTools;\n\tlet toolsProxy;\n",
    1,
)
defs = (
    '\tObject.defineProperty(globalThis, "tools", { value: toolsProxy, enumerable: true });\n'
    '\tObject.defineProperty(globalThis, "ALL_TOOLS", { value: allTools, enumerable: true });\n'
)
assert defs in s
s = s.replace(defs, "", 1)
s = s.replace("\treturn {\n\t\tsettle(", "\tconst api = {\n\t\tsettle(", 1)
assert s.rstrip().endswith("\t};\n})")
configure = (
    "\treturn function configure(toolsJson, globalsJson, storeJson) {\n"
    + block_a
    + "\t\tstored = new Map(Object.entries(parse(storeJson)));\n"
    + "\t\tfor (const [key, json] of stored) storedChars += key.length + json.length;\n"
    + defs
    + "\t\treturn api;\n\t};\n"
)
s = s.rstrip()[: -len("})")] + configure + "})"
sys.stdout.write(s)
