import { final, records, seed } from "../../rules.mjs";

export const note = "pi suite: Promise.all over two open calls' then-reactions (closures of the body)";

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  ["inline", [
    ['$dump.Fa7 = $djs.source("codemode.js", "d2f77c9ed6fbafb9", "", 1);\n', ""],
    ["}, $dump.Fa7)(", '}, $djs.source("codemode.js", "d2f77c9ed6fbafb9", "", 1))('],
  ]],
  // The callback runs once, right before the site, and what it makes is used once, at the
  // site: its expressions are written there, the closures where the body made them (in the
  // reactions, as the original's text has them), so no factory object and no Object.assign.
  ["callback", [
    ["$dump.s3 = {};\n", ""],
    ["$dump.ga7 = () => {\n$dump.p4 = Promise.prototype.then.call($dump.p1, $dump.s3.$f4, undefined);\n$dump.p5 = Promise.prototype.then.call($dump.p2, $dump.s3.$f6, undefined);\n$dump.p6 = Promise.all([$dump.p4, $dump.p5]);\n};\n", ""],
    ['Object.assign($dump.s3, { $f4: (0, () => order.push("a")), $f6: (0, () => order.push("b")) });\n$dump.ga7();\nawait $dump.p6;',
     'await Promise.all([Promise.prototype.then.call($dump.p1, () => order.push("a"), undefined), Promise.prototype.then.call($dump.p2, () => order.push("b"), undefined)]);'],
  ]],
  ["builtins.methods", [
    ['Promise.prototype.then.call($dump.p1, () => order.push("a"), undefined)', '$dump.p1.then(() => order.push("a"))'],
    ['Promise.prototype.then.call($dump.p2, () => order.push("b"), undefined)', '$dump.p2.then(() => order.push("b"))'],
  ]],
  // The open calls are used once each, in id order: written where they are used.
  ["inline", [
    ['$dump.p1 = $djs.resume(1, "call", ["call", "wait", "\\"a\\""], {"id":1});\n', ""],
    ['$dump.p2 = $djs.resume(2, "call", ["call", "wait", "\\"b\\""], {"id":2});\n', ""],
    ["$dump.p1.then(", '$djs.resume(1, "call", ["call", "wait", "\\"a\\""], {"id":1}).then('],
    ["$dump.p2.then(", '$djs.resume(2, "call", ["call", "wait", "\\"b\\""], {"id":2}).then('],
  ]],
  ["inline", [
    ["$dump.a7 = $djs.k(", "$djs.k("],
    ["1))();\n$dump.a7\n", "1))()\n"],
  ]],
  ["holder", [["globalThis.$dump = {};\n", ""]]],
  ["ws", [
    ["$djs.seed(67108866, 0);", "$djs.seed(67108866,0);"],
    ['$djs.resume(1, "call", ["call", "wait", "\\"a\\""], {"id":1})', '$djs.resume(1,"call",["call","wait","\\"a\\""],{"id":1})'],
    ['$djs.resume(2, "call", ["call", "wait", "\\"b\\""], {"id":2})', '$djs.resume(2,"call",["call","wait","\\"b\\""],{"id":2})'],
    ['}, $djs.source("codemode.js", "d2f77c9ed6fbafb9", "", 1))(', '},$djs.source("codemode.js","d2f77c9ed6fbafb9","",1))('],
  ]],
];
