// Awaits nested in literals: an array inside an object literal, an object inside an
// array, a spread and a computed key around them (plain dumps re-dump inside the
// literal at every idle point, 0203).
const calls = { calls: [await tools.echo(1), await tools.echo(2), await tools.echo(3)] };
console.log(JSON.stringify(calls));
const k = "key";
const mixed = [{ a: await tools.echo("a"), [k]: await tools.echo("k"), b: [await tools.echo("b1"), await tools.echo("b2")] }, ...[await tools.echo("s")]];
console.log(JSON.stringify(mixed));
const deep = { x: { y: [1, { z: await tools.echo("z"), w: await tools.echo("w") }] } };
console.log(JSON.stringify(deep));
return { calls: [await tools.echo("r1"), await tools.echo("r2")], n: 1 };
