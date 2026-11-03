// Nested loops: for-of in for-of in a while, each with its own iterator.
async function main() {
  let i = 0;
  while (i < 2) {
    for (const a of ["x", "y"]) {
      for (const b of "pq") print(i, a, b, await later(a + b));
    }
    i++;
  }
}
main();
