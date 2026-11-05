import { final, records, seed } from "../../rules.mjs";

export const note = "pi suite (next mode, a dump at every idle point), median size; Promise.all over an open call";

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  ["inline", [
    ['$dump.Fa3 = $djs.source("codemode.js", "54e8cfc5dc328017", "", 1);\n', ""],
    ["}, $dump.Fa3)(", '}, $djs.source("codemode.js", "54e8cfc5dc328017", "", 1))('],
  ]],
  // The Promise.all and the open call it waits on are used once: written at the site
  // (the same order: the call reopened, then Promise.all, then the await).
  ["inline", [
    ['$dump.p1 = $djs.resume(2, "call", ["call", "delay", "2"], {"id":2});\n$dump.p2 = Promise.all([1, $dump.p1, 3]);\n', ""],
    ["await $dump.p2;", 'await Promise.all([1, $djs.resume(2, "call", ["call", "delay", "2"], {"id":2}), 3]);'],
  ]],
  ["inline", [
    ["$dump.a3 = $djs.k(", "$djs.k("],
    ['($djs.host("tools"));\n$dump.a3\n', '($djs.host("tools"))\n'],
  ]],
  ["holder", [["globalThis.$dump = {};\n", ""]]],
  ["ws", [
    ["$djs.seed(1975298774, 1374804631, 4);", "$djs.seed(1975298774,1374804631,4);"],
    ['Promise.all([1, $djs.resume(2, "call", ["call", "delay", "2"], {"id":2}), 3])', 'Promise.all([1,$djs.resume(2,"call",["call","delay","2"],{"id":2}),3])'],
    ['}, $djs.source("codemode.js", "54e8cfc5dc328017", "", 1))(', '},$djs.source("codemode.js","54e8cfc5dc328017","",1))('],
  ]],
];
