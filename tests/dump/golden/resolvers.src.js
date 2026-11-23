// Resolving functions that user code keeps; a promise only they settle.
let keep;
const held = new Promise((res, rej) => { keep = { res, rej }; });
async function main() {
  const v = await later("go");
  keep.res(v + " resolved");
  print(await held);
}
held.then((v) => print("then", v));
main();
