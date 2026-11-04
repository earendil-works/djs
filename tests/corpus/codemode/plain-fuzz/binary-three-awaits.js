// Found by the plain-dump fuzzer (0208): a chain of binary operators with an await in
// every operand, dumped again inside the rewritten statement.
console.log(1 + (await tools.echo(1)) + (await tools.echo(2)) + (await tools.echo(3)));
return "done";
