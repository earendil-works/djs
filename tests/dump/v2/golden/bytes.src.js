// Binary data in flight (`$dump.bytes`): an ArrayBuffer and views over it.
const buf = new Uint8Array([1, 2, 250, 0, 7]).buffer;
const view = new DataView(buf, 1, 3);
async function main() {
  const u = new Uint16Array(4);
  u[1] = 513;
  await later(0);
  print(new Uint8Array(buf).join(), view.getUint8(1), Array.from(u).join());
}
main();
