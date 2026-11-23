// for await over a sync iterable (async-from-sync) and over an async generator.
async function* agen() {
  yield await later("a1");
  yield "a2";
}
async function main() {
  const out = [];
  for await (const x of ["s1", later("s2")]) out.push(x);
  for await (const x of agen()) out.push(await later(x + "!"));
  print(out.join());
}
main();
