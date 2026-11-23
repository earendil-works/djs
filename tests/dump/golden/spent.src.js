// Resolving functions whose promise has settled (`$dump.spent`): calling them does nothing.
let res, rej;
const p = new Promise((a, b) => { res = a; rej = b; });
res("first");
async function main() {
  await later(0);
  res("again");
  rej(new Error("late"));
  print(await p);
}
main();
