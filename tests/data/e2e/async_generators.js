// Async generators and for await (0041).
function S(v) {
  if (v && typeof v === "object") return "{" + Object.keys(v).map((k) => k + ":" + String(v[k])).join(",") + "}";
  return String(v);
}
const log = [];
async function* ag(n) {
  try {
    for (let i = 0; i < n; i++) {
      const r = yield i;
      log.push("r" + r);
    }
    return "end";
  } finally {
    log.push("ag fin");
  }
}
async function* deleg() {
  const v = yield* ag(2);
  log.push("v " + v);
  yield* [Promise.resolve("p1"), "s2"];
  return 7;
}
async function main() {
  for await (const x of ag(3)) log.push("x" + x);
  for await (const x of [Promise.resolve(1), 2]) log.push("y" + x);
  for await (const x of ag(10)) {
    if (x == 1) break;
    log.push("b" + x);
  }
  const it = deleg();
  log.push(S(await it.next()));
  log.push(S(await it.next("A")));
  log.push(S(await it.next("B")));
  log.push(S(await it.next()));
  log.push(S(await it.next()));
  log.push(S(await it.next()));
  const it2 = ag(5);
  await it2.next();
  log.push(S(await it2.return(Promise.resolve("rv"))));
  async function* thrower() {
    yield 1;
    throw new Error("gen err");
  }
  try {
    for await (const x of thrower()) log.push("t" + x);
  } catch (e) {
    log.push("caught " + e.message);
  }
  const o = { async *m() { yield await Promise.resolve(this.k); }, k: "K" };
  for await (const x of o.m()) log.push(x);
  return "main done";
}
main().then((v) => {
  log.push(v);
  console.log(log.join(" | "));
});
