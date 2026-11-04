// Helper classes in the body that closures waiting across awaits use: a class whose
// methods name it, reached from an arrow and from callbacks of Promise.all (0207).
class C { m() { return tools; } static make() { return new C(); } }
const f = async (x) => (await tools.echo(x)) + new C().m().echo.length;
console.log(await f(1));
const r = await Promise.all([1, 2].map(async (x) => (await tools.echo(x)) + (C.make().m() === tools)));
console.log(r);
