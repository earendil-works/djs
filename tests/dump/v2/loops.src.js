// Nested for-of loops around an await: a Map's iterator outside, a generator inside.
function* letters() {
  yield "a";
  yield "b";
}
async function main() {
  const m = new Map([["x", 1], ["y", 2]]);
  for (const [k, v] of m) {
    for (const l of letters()) {
      print(k, v, l, await later(k + l));
    }
  }
  print("end");
}
main();
