# The codemode workload set

Nine synthetic codemode bodies for the engine comparison of docs/guide/benchmarks.md
(js/bench/engines/run.ts, `scripts/bench-engines.sh`). Each reproduces the shape, size and
operation mix of one cluster of the bodies agents run in codemode sessions, with invented
data; work/research/0229/README.md says how the clusters were found and sized.

| workload | cluster (share of bodies) | what it does |
| --- | --- | --- |
| `sequential.js` | a few awaited calls (30%) | a read, an edit batch with large literal arguments, two commands |
| `fanout.js` | `Promise.all` (22%) | twelve reads and three checks at once, joined and cut to 40,000 characters |
| `media.js` | images (13%) | render a view, read the image, pass a 200 KB data URL to the host |
| `inspect.js` | no tools (9%) | group and search the tool catalog, a small computation |
| `loop.js` | a loop of calls (8%) | eight searches one after another, each formatted as Markdown |
| `text.js` | strings and regexes (8%) | parse grep-like output, group by file, scan files for markers and definitions |
| `json.js` | JSON reshaping (7%) | group 600 query rows, rewrite a package manifest for an edit |
| `ts.ts` | TypeScript (2%) | json.js's shape with interfaces, generics and annotations |
| `heavy.js` | the tail (largest 1%) | 97 calls: 96 reads in batches of eight with per-file regex work |

`NAME.py` is the same body in the Python subset monty runs, giving the same output.

The tools are stubs: `tools.ts` answers each call from the tool's name and arguments
(seeded by their hash, so concurrent calls may come in any order), and `record(body)` runs a
body in node to collect the answers it needs as JSON text. The engines get that table as
the string `__DATA` and their stub tools parse each answer (js/bench/engines/prelude.js,
prelude.py), the work a host bridge does. A body's result, its `text()` output plus its
return value, must be the same string on every engine as in node.
