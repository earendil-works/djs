// WeakRef/FinalizationRegistry observations under allocation pressure, across jobs.
const log = [];
const reg = new FinalizationRegistry((h) => log.push("fin" + h));
const refs = [];
for (let i = 0; i < 20; i++) { const o = { i, pad: new Array(50).fill(i) }; refs.push(new WeakRef(o)); reg.register(o, i); }
async function main() {
  for (let round = 0; round < 30; round++) {
    await null;
    let junk = [];
    for (let i = 0; i < 3000; i++) junk.push({ i, s: "x" + i });
    log.push(refs.filter((r) => r.deref() === undefined).length);
  }
}
main().then(() => console.log(log.join(" ")));
