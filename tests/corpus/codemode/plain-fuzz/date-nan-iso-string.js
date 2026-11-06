// An invalid date's `toISOString` after a dump: djs (as qjs) throws "RangeError: Date value
// is NaN", node "RangeError: Invalid time value"; plain-node reads node's as djs's (0237;
// plain-dump fuzzer seed 689441, frozen as regress/s0237-s689441.js).
const d = new Date(Date.UTC(2024, 1, await tools.echo("x")));
try {
  console.log(d.toISOString());
} catch (e) {
  console.log(String(e), e instanceof RangeError);
}
console.log(JSON.stringify(d), String(await tools.echo(d.getTime())));
