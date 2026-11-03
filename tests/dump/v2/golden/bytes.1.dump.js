"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [0]);
const buf = $dump.bytes("AQL6AAc=");
const view = new DataView(buf, 1, 3);
async function main() {
  const u = new Uint16Array(4);
  u[1] = 513;
  await later(0);
  print(new Uint8Array(buf).join(), view.getUint8(1), Array.from(u).join());
}
$dump.a2 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { u: new Uint16Array([0, 513, 0, 0]) } });
