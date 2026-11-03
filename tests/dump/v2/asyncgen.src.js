// Async generators: one awaiting inside its body with a request queued (a for-await
// waits on it), one suspended at a yield with nothing queued.
async function* pages(n) {
  for (let i = 0; i < n; i++) {
    const page = await later(`page ${i}`);
    yield page.toUpperCase();
  }
}
async function* ticker() {
  let k = 0;
  while (true) yield k++;
}
async function main() {
  const t = ticker();
  for await (const p of pages(3)) {
    const r = await t.next();
    print(p, r.value);
  }
  print("done");
}
main();
