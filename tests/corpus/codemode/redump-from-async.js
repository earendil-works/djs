// Array.fromAsync waiting on a tool, at every kind of await it has: an async iterator's
// next(), a mapped value, an array-like's element and its mapped value, the iterator's
// return() after a mapping failed; a sync iterable; an Array subclass (plain dumps write
// it as the runtime's fromAsync, 0215).
async function* g(n) { try { for (let i = 0; i < n; i++) yield await tools.echo(i); } finally { console.log("closed", await tools.echo("c")); } }
console.log(JSON.stringify(await Array.fromAsync(g(2))));
console.log(JSON.stringify(await Array.fromAsync(g(3), async (x, i) => x * 10 + i + await tools.echo(1))));
console.log(JSON.stringify(await Array.fromAsync({ length: 3, 0: tools.echo("a"), 1: "b", 2: tools.echo("c") })));
console.log(JSON.stringify(await Array.fromAsync({ length: 2, 0: "x", 1: "y" }, async (v, i) => v + await tools.echo(i))));
console.log(JSON.stringify(await Array.fromAsync([tools.echo(5), 6], (x) => x + 1)));
try {
  await Array.fromAsync(g(3), async (x) => { if (x === 1) throw new RangeError("map " + await tools.echo(x)); return x; });
} catch (e) { console.log("caught", e.name, e.message); }
class List extends Array {}
const l = await List.fromAsync(g(2));
console.log(l instanceof List, l.length, JSON.stringify([...l]));
