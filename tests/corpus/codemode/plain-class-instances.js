// An instance of a class that a plain dump makes in the continuation's block, held by a
// function that is not running: its prototype is set once the class exists (0209).
class C { m() { return typeof tools; } }
const o = new C();
async function f() { for (let i = 0; i < 1; i++) { let [] = [await tools.echo(1), o]; } }
const r = await tools.echo(1);
console.log(r, o instanceof C, o.m(), typeof f);
await f();
return await tools.echo(2);
