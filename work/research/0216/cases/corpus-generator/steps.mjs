import { final, records, seed, inline, methods, holder, scaffoldNames, whitespace } from "../../rules.mjs";

export const note = "corpus codemode/plain-fuzz/generator-capturing-for-of.js, link 1: a generator capturing a frame variable, for-of over it with an await; the difftest prelude's state (env) gets only the mechanical rules";

const G = "function* g(k) {\n  for (let i = 0; i < k; i++) yield i + n;\n}";
const K9 = "function* (k) {\n{ let $djs_f1 = true; for (let i = 0; $djs_f1 || (i < k); i++) { if ($djs_f1) { $djs_f1 = false; \nyield; } else yield i + n; } }\n}";

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  ["inline", inline],
  ["builtins.methods", methods],
  // The generator's prototype is g.prototype (the continuation's g, made with K): no
  // separate object (main's `{ __proto__: … }` is not g.prototype, the §9 finding), no
  // builtin lookup for it.
  ["record.encoding", [
    ["$dump.n10 = { __proto__: $dump.b0 };\n", ""],
    ['$djs.k($dump.s8.$Ka9, $dump.s8.g, "yield#1", $dump.n10);', '$djs.k($dump.s8.$Ka9, $dump.s8.g, "yield#1");'],
    ["$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;\n", ""],
  ]],
  // The callback makes the generator's activation, which the loop head (at load) reads:
  // written in K where the callback ran, g declared as the original declares it.
  ["callback", [
    ["$dump.s8 = {};\n", ""],
    ['$dump.ga11 = () => {\n$djs.k($dump.s8.$Ka9, $dump.s8.g, "yield#1");\n$dump.a9 = $dump.s8.$Ka9.call(undefined, 2);\n$dump.a9.next();\n};\n', ""],
    [`Object.assign($dump.s8, { g: ${G}, $Ka9: ${K9} });\nvar g = $dump.s8.g;\n$dump.ga11();\n`, `${G}\nconst $djs_g = $djs.k(${K9}, g, "yield#1")(2);\n$djs_g.next();\n`],
    ["$djs_L($dump.a9, $dump.b1)", "$djs_L($djs_g, $dump.b1)"],
  ]],
  // The loop head runs at load: the helper without a capture; `next` is the generator's
  // own (frozen builtins). The generator's C-style loop: the rest of the iteration (nothing
  // after the yield), then the loop from its update, as D2908 does for for-of (no closure
  // captures `i`, no break or continue).
  ["loop", [
    ["const $djs_L = $djs.L;\n", ""],
    ["$djs_L($djs_g, $dump.b1)", "$djs.L($djs_g)"],
    ['$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];\n', ""],
    ["{ let $djs_f1 = true; for (let i = 0; $djs_f1 || (i < k); i++) { if ($djs_f1) { $djs_f1 = false; \nyield; } else yield i + n; } }", "let i = 0;\nyield;\nfor (i++; i < k; i++) yield i + n;"],
  ]],
  // The rest of the iteration: `x` is dead; the compound assignment reads `n` again.
  ["reread", [["const x = 1;\n(n = 1 + await $dump.p7);", "n += await $dump.p7;"]]],
  ["inline", [
    ['$dump.p7 = Promise.prototype.then.call($djs.resume(1, "later", [1]), $dump.s2.$f4, undefined);\n', ""],
    ["n += await $dump.p7;", 'n += await $djs.resume(1, "later", [1]).then($dump.s2.$f4);'],
    ["$dump.a11 = $djs.k(", "$djs.k("],
    ['", "", 1))(tools);\n$dump.p12 = $dump.a11.then(', '", "", 1))(tools).then('],
  ]],
  ["names", holder],
  ["names", scaffoldNames],
  ["ws", whitespace],
  ["ws", [
    ['$djs.resume(1, "later", [1])', '$djs.resume(1,"later",[1])'],
    ['}, g, "yield#1")(2)', '},g,"yield#1")(2)'],
    ['}, $djs.source("codemode/plain-fuzz/generator-capturing-for-of.js", "6d50a03dd0a4eced", "", 1))(tools).then($d.s2.$f41, $d.s2.$f42);', '},$djs.source("codemode/plain-fuzz/generator-capturing-for-of.js","6d50a03dd0a4eced","",1))(tools).then($d.s2.$f41,$d.s2.$f42);'],
  ]],
];
