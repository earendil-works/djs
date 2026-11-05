// Found by the plain-dump fuzzer (0211): a `finally` whose end is dead (`if (true)
// continue`): the compiler keeps no completion for it, and the continuation needs none.
for (let i = 0; i < 2; i++) { try { console.log(i); } finally { console.log(await tools.echo(i)); if (true) continue; } }
async function f() { try { return 1; } finally { await tools.echo("f"); if (true) return 2; } }
console.log(await f());
