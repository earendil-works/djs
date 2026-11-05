// Found by the plain-dump fuzzer (0207, pjz): an async arrow mapped over an array that
// calls a function of the body; the re-dump writes the arrow's continuation where `id`
// is in scope.
function id(x) { return x; }
console.log(await Promise.all([1, 2].map(async (x) => await tools.echo(id(x)))));
const o = { value: 0 };
console.log(await Promise.all([3, 4].map(async (n) => { const r = await tools.echo(n); o.value = n; return r; })));
console.log(o.value);
