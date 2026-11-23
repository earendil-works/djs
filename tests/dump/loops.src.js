// Nested for-of loops around an await: a Map's iterator outside, a generator inside.
// loops.dump.js, written by hand, is this program at idle point 3: the outer loop at
// ["y", 2], the inner at "a".
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
