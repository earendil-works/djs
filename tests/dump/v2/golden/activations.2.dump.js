"djs-dump 2";
$dump.p1 = $dump.pending(2, "later", [2]);
async function twice(x) { return later(x * 2); }
async function args() { await later(arguments.length); return [...arguments].join("/"); }
async function shared() {
  let n = 1;
  const inc = () => ++n;
  await later(0);
  inc();
  return n;
}
async function main() { print(await twice(21), await args("a", "b"), await shared()); }
$dump.n2 = (function () { "use strict"; return arguments; })("a", "b");
$dump.a3 = $dump.resume(args, { at: "await#1", awaiting: $dump.p1, args: ["a", "b"], bindings: { "arguments": $dump.n2 } });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.a3, operands: [["call.callee", print], ["call.this", undefined], ["call.arg[0]", 42]] });
