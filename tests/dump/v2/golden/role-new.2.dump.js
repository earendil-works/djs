"djs-dump 2";
$dump.keep = (o, keys) => {
  for (const k of Reflect.ownKeys(o)) {
    if (!["length", "name", "prototype", "constructor"].includes(k) && !keys.includes(k)) delete o[k];
  }
  for (const k of keys) if (!Object.prototype.hasOwnProperty.call(o, k)) throw new TypeError("the class does not make " + String(k));
};
$dump.p1 = $dump.pending(2, "later", [2]);
class Thing { constructor(...a) { this.a = a; } }
async function main() {
  const x = new Thing(1, await later(2)).a.join();
  const y = new Thing(...[1], await later(2), 3).a.join();
  return x + " " + y;
}
$dump.a2 = $dump.resume(main, { at: "await#2", awaiting: $dump.p1, bindings: { x: "1,2", y: undefined }, operands: [["new.callee", Thing], ["new.args", [1]]] });
$dump.p3 = Promise.prototype.then.call($dump.a2, print, undefined);
$dump.keep(Thing, []);
$dump.keep(Thing.prototype, []);
