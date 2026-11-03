// Builtin state in flight, re-enacted by the exporter: then/catch/finally chains,
// resolving functions kept by user code, race and any, an async function returning a
// promise, string and Set iterators, for await over a sync iterable, a generator that
// has not started, the arguments object.
const log = [];
let keep;
const held = new Promise((res, rej) => {
  keep = { res, rej };
});
async function returnsPromise(x) {
  return later(x * 2);
}
async function* agen() {
  yield await later("g1");
  yield await later("g2");
}
function* lazy(a, b) {
  yield a;
  yield b;
}
async function withArgs() {
  const n = arguments.length;
  await later(n);
  return arguments[0] + "/" + n;
}
async function main() {
  const chain = later(1)
    .then((v) => v + 1)
    .catch(() => "never")
    .finally(() => log.push("finally ran"));
  const caught = later(new Error("boom"), true).catch((e) => "caught " + e.message);
  const race = Promise.race([later("fast"), held]);
  const any = Promise.any([later(new Error("x"), true), later("any ok")]);
  const ret = returnsPromise(21);
  const it = "héllo"[Symbol.iterator]();
  it.next();
  const sit = new Set(["s1", "s2", "s3"]).values();
  sit.next();
  const fresh = lazy("l1", "l2");
  const args = withArgs("first", 2, 3);
  const seen = [];
  for await (const x of ["fa", later("fb")]) seen.push(x);
  for await (const x of agen()) seen.push(x);
  print(await chain, await caught, await race, await any, await ret);
  keep.res("held resolved");
  print(await held, it.next().value, sit.next().value, fresh.next().value, await args);
  print(seen.join(), log.join());
}
main();
