import { final, records, seed } from "../../rules.mjs";

export const note = "examples/sleep-dump (the Sandbox's codemode body), the_sleep_example_dumps";

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  // The origin is used once: in the record.
  ["inline", [
    ['$dump.Fa6 = $djs.source("sleep.js", "b48bd687f864a8d6", "ms");\n', ""],
    ["}, $dump.Fa6)(", '}, $djs.source("sleep.js", "b48bd687f864a8d6", "ms"))('],
  ]],
  // The open call is awaited once, by step's continuation.
  ["inline", [
    ['$dump.p1 = $djs.resume(2, "tools.sleep", [5000], {"due":1793822825106});\n', ""],
    ["const r = await $dump.p1;", 'const r = await $djs.resume(2, "tools.sleep", [5000], {"due":1793822825106});'],
  ]],
  // The results array is the body's argument only.
  ["inline", [
    ["$dump.n4 = [];\n", ""],
    ["1793822820106, $dump.n4)", "1793822820106, [])"],
  ]],
  // step's activation is awaited once, at the body's site: its continuation is written
  // there (it runs at the same moment: the body's prefix runs to the site while the dump
  // loads). Its origin is then the body's own `step`.
  ["inline", [
    ['$dump.a3 = $djs.k(async function (label) {\nconst r = await $djs.resume(2, "tools.sleep", [5000], {"due":1793822825106});\nreturn label + ": " + r;\n}, $dump.n2)("first");\n', ""],
    ["results.push(await $dump.a3);", 'results.push(await $djs.k(async function (label) {\nconst r = await $djs.resume(2, "tools.sleep", [5000], {"due":1793822825106});\nreturn label + ": " + r;\n}, step)("first"));'],
  ]],
  // `step` is now only the body's argument: written there, the holder is unused.
  ["holder", [
    ['$dump.n2 = async function step(label, ms) {\n  console.log(label, "sleeps", ms, "ms");\n  const r = await tools.sleep(ms);\n  return label + ": " + r;\n};\n', ""],
    ["(5000, $dump.n2, ", '(5000, async function step(label, ms) {\n  console.log(label, "sleeps", ms, "ms");\n  const r = await tools.sleep(ms);\n  return label + ": " + r;\n}, '],
  ]],
  // The loop over an array iterator (no `return`, no break in the body, frozen builtins):
  // over the iterator itself, a parameter of K; the array without the element it handed
  // out (nothing else holds the array); no `$djs.I`, no capture of it.
  ["loop", [
    ['$dump.i5 = $djs.advance(Array.prototype.values.call(["first", "second"]), 1);\n', ""],
    ["const $djs_I = $djs.I;\nconst $djs_i1 = $dump.i5;\n", ""],
    ["for (const label of $djs_I($djs_i1))", "for (const label of $i)"],
    ["async function (ms, step, t0, results) {", "async function (ms, step, t0, results, $i) {"],
    ["1793822820106, [])", '1793822820106, [], ["second"].values())'],
  ]],
  ["holder", [["globalThis.$dump = {};\n", ""]]],
  ["ws", [
    ["$djs.seed(2135587861, -1640531527);", "$djs.seed(2135587861,-1640531527);"],
    ['$djs.resume(2, "tools.sleep", [5000], {"due":1793822825106})', '$djs.resume(2,"tools.sleep",[5000],{"due":1793822825106})'],
    ['$djs.source("sleep.js", "b48bd687f864a8d6", "ms"))(5000, async', '$djs.source("sleep.js","b48bd687f864a8d6","ms"))(5000,async'],
    ["}, step)(", "},step)("],
    ['}, 1793822820106, [], ["second"].values())', '},1793822820106,[],["second"].values())'],
  ]],
  // The run's promise (the one handle) is the completion value: the call itself.
  ["inline", [
    ["$dump.a6 = $djs.k(", "$djs.k("],
    ['.values());\n$dump.a6\n', ".values())\n"],
  ]],
];
