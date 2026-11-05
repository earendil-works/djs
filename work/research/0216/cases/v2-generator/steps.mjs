import { final, records, seed } from "../../rules.mjs";

export const note = "tests/dump/v2/generator, link 2: a generator suspended at a yield inside a for loop, driven by an async function waiting inside a while loop";

const COUNTER = "function* counter(limit) {\n  let total = 0;\n  for (let i = 0; i < limit; i++) {\n    const got = yield i;\n    total += got;\n  }\n  return total;\n}";

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  // The generator's prototype is counter.prototype: no object of its own (main's is a
  // different object, so `Object.getPrototypeOf(g) === counter.prototype` is false after a
  // load: a bug), no builtin lookup.
  ["record.encoding", [
    ["$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;\n", ""],
    ["$dump.n3 = { __proto__: $dump.b0 };\n", ""],
    [" }, $dump.n3)(4, 0);", " })(4, 0);"],
  ]],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  // A global generator function written as the original declares it (main is).
  ["decl", [[`var counter = ${COUNTER};`, COUNTER]]],
  // Loops whose bodies cannot break or continue: the rest of the current iteration, then
  // the loop as written (D2908's rule for for-of, for `while` and C-style `for`, whose
  // loop variable no closure captures): no `$djs_f1` placeholder scaffolds.
  ["loop", [
    ["{ let $djs_f1 = true; for (let i = 1; $djs_f1 || (i < limit); i++) { if ($djs_f1) { $djs_f1 = false; \n{\nconst got = yield;\ntotal += got;\n}\n } else {\n    const got = yield i;\n    total += got;\n  } } }",
     "let i = 1;\n{\nconst got = yield;\ntotal += got;\n}\nfor (i++; i < limit; i++) {\n    const got = yield i;\n    total += got;\n  }"],
    ['{ let $djs_f1 = true; while ($djs_f1 || (!r.done)) { if ($djs_f1) { $djs_f1 = false; {\nconst v = await $dump.p1;\nprint("got", v);\nr = g.next(v);\n}\n } else {\n    const v = await later(r.value * 10);\n    print("got", v);\n    r = g.next(v);\n  } } }',
     '{\nconst v = await $dump.p1;\nprint("got", v);\nr = g.next(v);\n}\nwhile (!r.done) {\n    const v = await later(r.value * 10);\n    print("got", v);\n    r = g.next(v);\n  }'],
  ]],
  ["inline", [
    ['$dump.p1 = $djs.resume(3, "later", [10]);\n', ""],
    ["const v = await $dump.p1;", 'const v = await $djs.resume(3, "later", [10]);'],
  ]],
  // The generator is used twice (primed, then main's argument): a block-scoped constant
  // instead of the global holder (nothing to delete, nothing global).
  ["holder", [
    ["globalThis.$dump = {};\n", ""],
    ["$dump.a2 = $djs.k(function* (limit, total) {", "{const $g = $djs.k(function* (limit, total) {"],
    ["$dump.a2.next();\n$dump.a4 = $djs.k(", "$g.next();\n$djs.k("],
    ["}, main)($dump.a2, { value: 1, done: false });\n", "}, main)($g, { value: 1, done: false })}\n"],
  ]],
  ["ws", [
    ["$djs.seed(2135587861, -1640531527);", "$djs.seed(2135587861,-1640531527);"],
    ['$djs.resume(3, "later", [10])', '$djs.resume(3,"later",[10])'],
    ['}, counter, "yield#1")(4, 0);', '},counter,"yield#1")(4,0);'],
    ["}, main)($g, { value: 1, done: false })}", "},main)($g,{value:1,done:false})}"],
    ["{const $g = $djs.k(", "{const $g=$djs.k("],
  ]],
];
