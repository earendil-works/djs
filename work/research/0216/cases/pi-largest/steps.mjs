import { final, records, seed } from "../../rules.mjs";

export const note = "pi suite, largest dump; an object literal half built at the site";

export const steps = [
  ["final", final],
  ["record.layout", (t) => records(t, "layout")],
  ["record.encoding", (t) => records(t, "positional")],
  ["seed.next", seed],
  ["inline", [
    ['$dump.Fa2 = $djs.source("codemode.js", "f593729dd2bd2a94", "", 1);\n', ""],
    ["}, $dump.Fa2, 2)(", '}, $djs.source("codemode.js", "f593729dd2bd2a94", "", 1), 2)('],
  ]],
  ["inline", [
    ['$dump.p1 = $djs.resume(2, "call", ["call", "my-tool", undefined], {"id":2});\n', ""],
    ["await $dump.p1,", 'await $djs.resume(2, "call", ["call", "my-tool", undefined], {"id":2}),'],
  ]],
  // The object literal evaluated before the site is written as the literal again: its
  // `all` is an identifier that holds the saved value at load (a host global the host binds
  // again by name: D2905's rule for identifiers, extended from call operands to every
  // operand), so no temporary, no `$djs.D`, no comma expression.
  ["reread", [
    ['$dump.a2_0 = { all: $djs.host("ALL_TOOLS") };\n', ""],
    ["const $djs_D = $djs.D;\nconst $djs_t0 = $dump.a2_0;\n", ""],
    ['return ($djs_D($djs_t0, "calls", ["dash", ', 'return { all: ALL_TOOLS, calls: ["dash", '],
    ["await tools.mcp__docs__search()]), $djs_t0);", "await tools.mcp__docs__search()] };"],
  ]],
  ["inline", [
    ["$dump.a2 = $djs.k(", "$djs.k("],
    ['($djs.host("tools"));\n$dump.a2\n', '($djs.host("tools"))\n'],
  ]],
  ["holder", [["globalThis.$dump = {};\n", ""]]],
  ["ws", [
    ["$djs.seed(2033933741, -1539143467);", "$djs.seed(2033933741,-1539143467);"],
    ['$djs.resume(2, "call", ["call", "my-tool", undefined], {"id":2})', '$djs.resume(2,"call",["call","my-tool",undefined],{"id":2})'],
    ['}, $djs.source("codemode.js", "f593729dd2bd2a94", "", 1), 2)(', '},$djs.source("codemode.js","f593729dd2bd2a94","",1),2)('],
  ]],
];
