// Found by the plain-dump fuzzer (0208): awaits in two parts of one template literal.
console.log(`${await tools.echo("a")}-${await tools.echo("b")}`);
return "done";
