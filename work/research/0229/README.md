# 0229: how the codemode workload set was derived

The benchmarks page runs nine synthetic workloads (tests/bench/codemode). They reproduce the
shapes, sizes and operation mix of the codemode bodies agents ran in the owner's pi sessions,
with invented data. **Nothing from the sessions is in the repository**: no body, path, file
content, prompt, tool output, tool or server name. This note gives counts and distributions
only.

## Extraction

`extract.py` (caps: 200 bodies per session file, 64 KiB per body) found the session files
that mention the codemode tool (1,344 of 10,178 JSONL files, 7.2 GB in 3,758 directories;
listed with `rg -l`) and wrote each codemode call's `code` with only
the size of its result text, whether it failed, and per tool call it made (from the
result's `details.calls`) the name, argument length and duration, to
work/scratch/0229/private/ (gitignored). Result: 2,945 bodies from 377 session files,
2026-09 to 2026-11 (84% from October); 87 failed.

## What the bodies look like (all 2,945)

- Size: median 515 characters; p25 260, p75 1,088, p90 2,219, p99 7,501.
- Tool calls at run time: median 3; p25 1, p75 4, p90 7, p99 about 100. 291 (10%) call
  no tool. Median 1 distinct tool per body (p99 4).
- Result text returned to the model: median 1.8 KB; p75 12 KB, p90 29 KB, p99 40 KB (the
  output cap).
- Which tools: file reads in 42% of the bodies, shell commands 37%, namespaced (MCP) tools
  29%, edits 11%, writes 4%, others 5%. 12% of the results carry an image.
- Constructs (share of bodies): `await` 98%, arrow functions 46%, template literals 42%,
  `text()` output 39%, `.map` 34%, `for...of` 29%, `Promise.all`/`allSettled` 28%,
  `.slice` 21%, `JSON.stringify` 14%, `image()` 13%, regex literals 7%, `JSON.parse` 6%,
  TypeScript annotations 2%, classes 0%.

Engine work is small: bodies are short, make a few calls and spend their time in
startup, the tool bridge (arguments out, results in), parsing literal-heavy source and
assembling strings from tool output.

## Clusters

`cluster.py` assigns each body to the first rule it matches: TypeScript syntax; no tool
call; `image()`; two or more string/regex operations; two or more JSON/collection
operations; `Promise.all`; a loop; else sequential.

| cluster | bodies | share | chars p25/50/90 | calls p25/50/90 | result chars p25/50/90 | workload |
| --- | ---: | ---: | --- | --- | --- | --- |
| sequential | 881 | 30% | 316 / 529 / 1,898 | 1 / 2 / 4 | 280 / 1,440 / 16,891 | sequential.js |
| fan-out | 658 | 22% | 370 / 541 / 1,590 | 3 / 4 / 8 | 2,924 / 13,680 / 40,273 | fanout.js |
| media | 394 | 13% | 168 / 210 / 632 | 2 / 3 / 5 | 285 / 404 / 907 | media.js |
| no tools | 274 | 9% | 61 / 97 / 285 | 0 / 0 / 0 | 1,003 / 4,414 / 15,802 | inspect.js |
| loop | 240 | 8% | 412 / 662 / 1,912 | 3 / 5 / 9 | 1,032 / 16,566 / 40,274 | loop.js |
| text | 240 | 8% | 908 / 1,619 / 5,969 | 2 / 3 / 61 | 171 / 728 / 11,662 | text.js |
| json | 195 | 7% | 836 / 1,279 / 3,785 | 1 / 3 / 17 | 346 / 1,308 / 15,378 | json.js |
| TypeScript | 63 | 2% | 1,314 / 2,213 / 6,117 | 2 / 3 / 6 | 84 / 318 / 23,170 | ts.ts |
| (tail) | | p99 | | ~100 | 40,000 | heavy.js |

What each cluster's bodies do, in general terms (from reading samples, not quoted):
sequential bodies await one to four calls, often with a large literal argument (an edit
batch with multi-line old/new text, a long command), and print the results; fan-out
bodies read a list of files or run several commands at once and join the results under
headers, cut to the output budget; media bodies render or capture a view and hand the
image to the host; tool-free bodies list or search the tool catalog or compute a small
number; loops make one call per item (searches, reads) and format each result as
Markdown; text bodies split, match and replace over command output and file contents;
JSON bodies reshape query results or rewrite package manifests; TypeScript bodies are
the JSON and text shapes with type annotations.

## The workloads

Each workload is one body per cluster, sized at or a little above its cluster's median
(engine work shows; still a typical body), plus one tail case:

| workload | body chars | tool calls | tool data (JSON) | output chars |
| --- | ---: | ---: | ---: | ---: |
| sequential | 2,305 | 4 | 15 KB | 2,907 |
| fanout | 1,166 | 15 | 155 KB | 40,028 |
| media | 359 | 2 | 184 KB | 78 |
| inspect | 596 | 0 | 7 KB | 401 |
| loop | 1,185 | 8 | 51 KB | 16,002 |
| text | 1,414 | 5 | 64 KB | 1,242 |
| json | 1,748 | 3 | 145 KB | 2,787 |
| ts | 2,154 | 1 | 144 KB | 1,521 |
| heavy | 1,477 | 97 | 953 KB | 10,313 |

The stub tools (tests/bench/codemode/tools.ts) answer from the tool's name and arguments:
source-like files (imports, functions, comments with TODO/FIXME, 40 to 940 lines),
grep-like and test-runner output, git status and diffstat, package manifests, search
results with prose, query rows, a PNG-like image as a data URL. Words come from small
fixed lists; nothing is taken from the sessions.

Each body has a Python translation for monty (`*.py`), written by hand; all nine give the
same output string as the JavaScript (the harness checks it).

## Engines and method

See js/bench/engines/run.ts's header, scripts/bench-engines.sh and D3110–D3114.

## Findings

- djs is fastest at startup (a new instance and VM) and on the small bodies, and smallest
  in memory among the engines that start under 1 MiB, but slower than QuickJS on `heavy`:
  6.5 ms of its 20 ms there are 30,000 `Math.max` calls (about 200 ns each, against 20 ns in
  QuickJS: builtins/math.ts takes rest arguments into an array and loops in bytecode).
  Reported as a follow-up.
- Bellard's QuickJS is fastest on the `loop` workload (regex replace over prose): its
  regex engine beats QuickJS-NG's and djs's there.
- quickjs-emscripten's builds start with 16 MiB of linear memory (emscripten's initial
  heap), whatever the workload.
- monty's module is 24 MB (7.5 MB gzip); its regex and string splitting make it the
  fastest on `heavy`.
