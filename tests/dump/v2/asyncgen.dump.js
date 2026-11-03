"djs-dump 2";
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
$dump.ticker = $dump.resume(ticker, { at: "yield#1", bindings: { k: 1 } });
$dump.q1 = new Promise(() => {});
$dump.pages = $dump.resume(pages, {
  at: "await#1",
  awaiting: $dump.pending(2, "later", ["page 1"]),
  args: [3],
  bindings: { n: 3, i: 1, page: $dump.TDZ },
  queue: [{ type: "next", value: undefined, promise: $dump.q1 }],
});
$dump.handle("main", $dump.resume(main, {
  at: "for-await.next#1",
  awaiting: $dump.q1,
  bindings: { t: $dump.ticker, p: "PAGE 0" },
  loops: [{ kind: "for-await", iterator: $dump.pages, next: $dump.pages.next }],
}));
