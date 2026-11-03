"djs-dump 2";
$dump.p1 = $dump.pending(1, "later", [42]);
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
$dump.p2 = new Promise((res, rej) => { $dump.r3 = [res, rej]; });
$dump.p4 = Promise.prototype.then.call($dump.p1, $dump.r3[0], $dump.r3[1]);
$dump.a5 = $dump.resume(main, { at: "await#1", awaiting: $dump.p2, operands: [["call.callee", print], ["call.this", undefined]] });
