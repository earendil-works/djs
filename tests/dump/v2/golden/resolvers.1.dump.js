"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", ["go"]);
$dump.p2 = new Promise((res, rej) => { $dump.r3 = [res, rej]; });
let keep = { res: $dump.r3[0], rej: $dump.r3[1] };
const held = $dump.p2;
async function main() {
  const v = await later("go");
  keep.res(v + " resolved");
  print(await held);
}
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { v: undefined } });
$dump.p5 = Promise.prototype.then.call($dump.p2, (v) => print("then", v), undefined);
