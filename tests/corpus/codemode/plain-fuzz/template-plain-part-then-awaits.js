// Found by the plain-dump fuzzer (0211): a template literal whose first part has no
// await, then two awaits, dumped again at the second.
let n = 0;
console.log(`${"a" + n}-${await tools.echo(n)}:${await tools.echo(" ")}`);
return "done";
