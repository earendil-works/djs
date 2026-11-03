// An async function returning a promise, one reading its arguments object, and a
// frame sharing a binding with a closure.
async function twice(x) { return later(x * 2); }
async function args() { await later(arguments.length); return [...arguments].join("/"); }
async function shared() {
  let n = 1;
  const inc = () => ++n;
  await later(0);
  inc();
  return n;
}
async function main() { print(await twice(21), await args("a", "b"), await shared()); }
main();
