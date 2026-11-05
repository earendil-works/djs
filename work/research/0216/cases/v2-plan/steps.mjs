import { final, records, seed } from "../../rules.mjs";

export const note = "tests/dump/v2/plan (docs/continuation-dumps.md's example): closures sharing a frame variable, a nested continuation, Promise.all, a loop";

const K5 = `async function (path) {
const body = await $dump.p1;
const n = body.split("\\n").filter((l) => l.includes("TODO")).length;
seen.set(path, n);
return n;
}`;
const TODOS = `async function todos(path) {
    const body = await tools.read({ path });
    const n = body.split("\\n").filter((l) => l.includes("TODO")).length;
    seen.set(path, n);
    return n;
  }`;

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  // The callback starts todos' continuation and the Promise.all; both are used once, at
  // main's site, right after the callback ran: written there. todos is declared as the
  // original declares it (a function declaration of K), its continuation is made where it
  // is awaited, so no factory object, no Object.assign, no `var todos = $dump.s2.todos`,
  // no `.call(undefined, …)`.
  ["callback", [
    ["$dump.s2 = {};\n", ""],
    ["$dump.ga9 = () => {\n$djs.k($dump.s2.$Ka5, $dump.s2.todos);\n$dump.a5 = $dump.s2.$Ka5.call(undefined, \"src/b.ts\");\n$dump.p6 = Promise.all([2, $dump.a5]);\n};\n", ""],
    [`Object.assign($dump.s2, { todos: ${TODOS}, $Ka5: ${K5} });\nvar todos = $dump.s2.todos;\n$dump.ga9();\n`, `${TODOS}\n`],
    ["report.push(await $dump.p6);", `report.push(await Promise.all([2, $djs.k(${K5}, todos)("src/b.ts")]));`],
  ]],
  ["inline", [
    ['$dump.p1 = $djs.resume(3, "later", ["fine"]);\n', ""],
    ["const body = await $dump.p1;", 'const body = await $djs.resume(3, "later", ["fine"]);'],
  ]],
  ["inline", [
    ['$dump.n3 = new Map([["src/a.ts", 2]]);\n', ""],
    ["const seen = $dump.n3;", 'const seen = new Map([["src/a.ts", 2]]);'],
    ["$dump.n7 = [];\n", ""],
    ['"src/d.ts"], $dump.n7);', '"src/d.ts"], []);'],
  ]],
  // The loop over an array iterator: the iterator itself (no `return`, no break), a
  // parameter of K; its array without the element handed out (only the iterator holds it).
  ["loop", [
    ['$dump.n4 = ["src/a.ts", "src/b.ts"];\n', ""],
    ['$dump.i8 = $djs.advance(Array.prototype.values.call([$dump.n4, ["src/c.ts", "src/d.ts"]]), 1);\n', ""],
    ["const $djs_I = $djs.I;\n", ""],
    ["const $djs_i1 = $dump.i8;\n", ""],
    ["for (const batch of $djs_I($djs_i1))", "for (const batch of $i)"],
    ["async function (files, report) {", "async function (files, report, $i) {"],
    ['"src/d.ts"], []);', '"src/d.ts"], [], [["src/c.ts", "src/d.ts"]].values());'],
  ]],
  ["builtins.methods", [
    ["$dump.p10 = Promise.prototype.then.call($dump.a9, (r) => print(JSON.stringify(r)), undefined);", "$dump.p10 = $dump.a9.then((r) => print(JSON.stringify(r)));"],
  ]],
  // The run's promise is used once, by the program's `then` (whose result nothing holds).
  ["inline", [
    ["$dump.a9 = $djs.k(", "$djs.k("],
    ['.values());\n$dump.p10 = $dump.a9.then(', ".values()).then("],
  ]],
  ["holder", [["globalThis.$dump = {};\n", ""]]],
  ["ws", [
    ["$djs.seed(2135587861, -1640531527);", "$djs.seed(2135587861,-1640531527);"],
    ['$djs.resume(3, "later", ["fine"])', '$djs.resume(3,"later",["fine"])'],
    ['}, todos)("src/b.ts")', '},todos)("src/b.ts")'],
    ['}, main, 2)(["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"], [], [["src/c.ts", "src/d.ts"]].values()).then((r) => print(JSON.stringify(r)));\n',
     '},main,2)(["src/a.ts","src/b.ts","src/c.ts","src/d.ts"],[],[["src/c.ts","src/d.ts"]].values()).then((r) => print(JSON.stringify(r)))\n'],
  ]],
];
