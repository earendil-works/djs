"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [2]);
class Thing { constructor(...a) { this.a = a; } }
async function main() {
  const x = new Thing(1, await later(2)).a.join();
  const y = new Thing(...[1], await later(2), 3).a.join();
  return x + " " + y;
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { x: undefined, y: undefined, "new.callee@36": Thing, "new.arg@46": 1 } });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$dump.keep(Thing, []);
$dump.keep(Thing.prototype, []);
