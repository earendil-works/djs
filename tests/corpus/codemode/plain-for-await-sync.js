// `for await` over sync iterators across re-dumps (0209): the dump runtime's iterator and
// its step in flight are written as `$djs.fromSync`/`$djs.closeThrow` again, also where
// the step is done (a generator's returned promise) or closes the iterator (a rejection).
const out = [];
for await (const x of [tools.echo(1), tools.echo(2), tools.echo(3)]) {
  out.push(x);
  out.push(await tools.echo("body" + x));
}
function* gen() {
  try {
    yield tools.echo("g1");
    yield tools.fail({ message: "no" });
    yield 3;
  } finally {
    console.log("closed");
  }
}
try {
  for await (const y of gen()) out.push(y);
} catch (e) {
  out.push("caught " + e.message);
}
function* ret() {
  yield tools.echo("r1");
  return tools.echo("end");
}
for await (const z of ret()) out.push(z);
console.log(JSON.stringify(out));
return out.length;
