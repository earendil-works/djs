// Found by the plain-dump fuzzer (0207, pjz, seed 9828): a function's text has a regular
// expression in a template's substitution (`${/a(b)?/g.source}`); the dump's reference
// scan read it as a division and missed the references after it, so a value still read
// was written as a statement and the load read `undefined`.
async function f2(a3) {
  console.log("p7", await tools.echo(a3));
  console.log("u5", await tools.echo("bye"));
  const v13 = `${await tools.echo("ok")}-${(await tools.echo([1, 9]))[1]}:${/a(b)?/g.source}`;
  return v13;
}
console.log(await f2(1));
