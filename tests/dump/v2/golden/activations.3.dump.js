"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(3, "later", [0]);
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
$dump.s2 = (() => {
  let n = 1;
  return { inc: () => ++n, $c4: $dump.cell(() => n) };
})();
$dump.a3 = $dump.resume(shared, { at: "await#1", awaiting: $dump.p1, slots: { n: $dump.s2.$c4, inc: $dump.s2.inc } });
$dump.a4 = $dump.resume(main, { at: "await#3", awaiting: $dump.a3, slots: { "call.callee@24": print, "call.this@24": undefined, "call.arg@30": 42, "call.arg@47": "a/b" } });
