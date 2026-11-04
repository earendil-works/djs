// Found by the plain-dump fuzzer (0208): awaits in a call's computed callee and in its
// argument, dumped again at the second.
const fs = [(x) => x + 1];
console.log(fs[await tools.echo(0)](await tools.echo(2)));
return "done";
