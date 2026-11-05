import { final, records, seed } from "../../rules.mjs";
import { statements } from "../../scan.mjs";

export const note = "tests/dump/v2/operands, the largest v2 dump: 17 activations stopped inside expressions (calls, new, members, literals, assignments, templates, super)";

const K = (fn, body, args = "") => [`async function () {\n${body}`, fn, args];

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  // No member was deleted from these classes since their evaluation: nothing to keep.
  ["keep", (t) => t.replace(/^\$djs\.keep\(.*\);\n/gm, "")],
  // Operands evaluated before the site, written as the original's text where it gives the
  // saved value on the dumped state: identifiers that hold it (globals, K's parameters), a
  // data property of an ordinary object the dump has finished writing before K runs (`o.f`),
  // a fresh literal nothing else holds (`...[1]`, `[1, , `), the old value of a compound
  // assignment. No temporaries, no `$djs.R/C/D/Q`, no `$dump.aN_k` copies.
  ["reread", [
    ["$dump.a19_0 = o;\n", ""],
    ["const $djs_R = $djs.R;\nconst $djs_t0 = $dump.n18, $djs_t1 = $dump.a19_0;\nreturn $djs_R($djs_t0, $djs_t1, [1, await $dump.p1, 3]);", "return o.f(1, await $dump.p1, 3);"],
    ["$dump.a20_0 = o;\n", ""],
    ["const $djs_R = $djs.R, $djs_Q = $djs.Q;\nconst $djs_t0 = $dump.n18, $djs_t1 = $dump.a20_0, $djs_t2 = [1];\nreturn $djs_R($djs_t0, $djs_t1, $djs_Q($djs_t2, await $dump.p2, 3));", "return o.f(...[1], await $dump.p2, 3);"],
    ["$dump.a22_0 = Thing;\n", ""],
    ["const $djs_C = $djs.C;\nconst $djs_t0 = $dump.a22_0;\nreturn $djs_C($djs_t0, [1, await $dump.p4]).a.join();", "return new Thing(1, await $dump.p4).a.join();"],
    ["$dump.a23_0 = Thing;\n", ""],
    ["const $djs_C = $djs.C, $djs_Q = $djs.Q;\nconst $djs_t0 = $dump.a23_0, $djs_t1 = [1];\nreturn $djs_C($djs_t0, $djs_Q($djs_t1, await $dump.p5)).a.join();", "return new Thing(...[1], await $dump.p5).a.join();"],
    ["$dump.n24 = { x: 9 };\n", ""],
    ["async function () {\nconst $djs_t0 = $dump.n24;\nreturn $djs_t0[(await $dump.p6)];\n}, member)()", "async function (t) {\nreturn t[await $dump.p6];\n}, member)({ x: 9 })"],
    ["$dump.a26_0 = { a: 1 };\n", ""],
    ['async function () {\nconst $djs_D = $djs.D;\nconst $djs_t0 = $dump.a26_0;\nreturn JSON.stringify(($djs_D($djs_t0, "z", await $dump.p7), $djs_t0));\n}, key)()', 'async function (k) {\nreturn JSON.stringify({ a: 1, [k]: await $dump.p7 });\n}, key)("z")'],
    ["$dump.a27_0 = { a: 1 };\n", ""],
    ['const $djs_D = $djs.D;\nconst $djs_t0 = $dump.a27_0;\nreturn JSON.stringify(($djs_D($djs_t0, "c", await $dump.p8), $djs_t0));', "return JSON.stringify({ a: 1, c: await $dump.p8 });"],
    ["const $djs_Q = $djs.Q;\nconst $djs_t0 = [1, , ];\nreturn $djs_Q($djs_t0, await $dump.p10).length;", "return [1, , await $dump.p10].length;"],
    ["async function () {\nreturn (5 * (await $dump.p11));\n}, left)()", "async function (a) {\nreturn a * (await $dump.p11);\n}, left)(5)"],
    ["$dump.n31 = {};\n", ""],
    ["const $djs_t0 = $dump.n31;\n($djs_t0.p = await $dump.p12);", "t.p = await $dump.p12;"],
    ["}, prop)($dump.n31)", "}, prop)({})"],
    ["$dump.n33 = {};\n", ""],
    ['async function (t) {\nconst $djs_t0 = $dump.n33;\n($djs_t0["q"] = await $dump.p13);', "async function (t, k) {\nt[k] = await $dump.p13;"],
    ["}, index)($dump.n33)", '}, index)({}, "q")'],
    ["(n = 1 + await $dump.p14);", "n += await $dump.p14;"],
    ["$dump.n36 = { r: 1 };\n", ""],
    ["const $djs_t0 = $dump.n36;\n($djs_t0.r = 1 + await $dump.p15);", "t.r += await $dump.p15;"],
    ["}, compoundProp)($dump.n36)", "}, compoundProp)({ r: 1 })"],
    ['return `a${"1"}b${await $dump.p16}c`;', "return `a${1}b${await $dump.p16}c`;"],
    ["return super[(await $dump.p17)](x);", "return super[await $dump.p17](x);"],
  ]],
  // Written but never read (the super method's home, which its object literal gives).
  ["unused", [["$dump.a40_0 = Base.prototype;\n", ""]]],
  // `o.f` is no longer read through `$dump.n18`: the method is written in its object.
  ["inline", [
    ["$dump.n18 = $djs.fn({ f(a, b, c) { return [this.k, a, b, c].join(); } });\nconst o = { k: 1, f: $dump.n18 };", "const o = { k: 1, f(a, b, c) { return [this.k, a, b, c].join(); } };"],
  ]],
  // The open calls, each awaited once, in id order: written at their sites.
  ["inline", (t) => {
    for (const m of [...t.matchAll(/^\$dump\.(p\d+) = (\$djs\.resume\(.*\));\n/gm)]) {
      t = t.replace(m[0], "");
      const re = new RegExp(`\\$dump\\.${m[1]}(?!\\d)`);
      if (!re.test(t)) throw new Error(m[1]);
      t = t.replace(re, m[2]);
    }
    return t;
  }],
  // The activations are used once, by main's Promise.all, which main's site awaits: all
  // written there, in the same order.
  ["inline", (t) => {
    const all = /^\$dump\.p41 = Promise\.all\(\[(.*)\]\);\n/m.exec(t);
    const names = all[1].split(", ").map((s) => s.replace("$dump.", ""));
    const ks = [];
    for (const n of names) {
      const st = statements(t).map(([a, b]) => t.slice(a, b)).find((x) => x.startsWith(`$dump.${n} = `));
      if (!st) throw new Error(n);
      ks.push(st.slice(`$dump.${n} = `.length).replace(/;\n$/, ""));
      t = t.replace(st, "");
    }
    t = t.replace(all[0], "");
    t = t.replace("const rs = await $dump.p41;", `const rs = await Promise.all([${ks.join(", ")}]);`);
    return t.replace("$dump.n39 = { __proto__: Child.prototype };\n", "").replace(".call($dump.n39, 7)", ".call({ __proto__: Child.prototype }, 7)");
  }],
  ["inline", [["$dump.a42 = $djs.k(", "$djs.k("]]],
  ["holder", [["globalThis.$dump = {};\n", ""]]],
  ["ws", (t) => t
    .replace("$djs.seed(2135587861, -1640531527);", "$djs.seed(2135587861,-1640531527);")
    .replace(/\$djs\.resume\((\d+), "later", \[/g, '$djs.resume($1,"later",[')
    .replace(/\n\}, ([\w.]+)\)\(/g, "\n},$1)(")
    .replace(/\)\(\), \$djs\.k\(/g, ")(),$djs.k(")
    .replace(/\), \$djs\.k\(/g, "),$djs.k(")
    .replace(/\n\} \}\)\.K, Child\.prototype\.m\)/, "\n} }).K,Child.prototype.m)")
    .replace(".call({ __proto__: Child.prototype }, 7)", ".call({__proto__:Child.prototype},7)")
    .replace(/\}, main\)\(\);\n$/, "},main)()\n")],
];
