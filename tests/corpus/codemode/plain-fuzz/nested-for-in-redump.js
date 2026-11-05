// difftest: plain-xfail a re-dump reads the remaining keys of the inner for-in that K runs verbatim as empty: iterations are lost
// Found by the plain-dump fuzzer (0211): a `for in` inside another loop, dumped at its
// last key and dumped again in the next outer iteration.
let count = 0;
for (const a of [1, 2]) {
  for (const b in a === 1 ? { c: 3 } : { a: 1, b: 2, c: 3 }) {
    await tools.echo(b);
    count++;
  }
}
return count;
