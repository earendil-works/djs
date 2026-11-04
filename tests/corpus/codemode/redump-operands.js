// Re-dumps inside one statement with several awaits (plain dumps, 0207): binary operators,
// a template, a callee whose key awaited, an assignment to a member.
const x = (await tools.echo(1)) + (await tools.echo(2));
const o = {};
o.a = (await tools.echo(3)) + (await tools.echo(4));
console.log(x, o.a, 1 + (await tools.echo(1)) + (await tools.echo(2)) + (await tools.echo(3)));
console.log(`${await tools.echo("a")}-${await tools.echo("b")}`);
const fs = [(v) => "f" + v];
console.log(fs[await tools.echo(0)](await tools.echo(2)));
const f = async (v) => { let s = 0; for (const i of [1, 2]) s += await tools.echo(i); return s + v; };
o["k"] = await f(await tools.echo(1));
console.log(o.k);
