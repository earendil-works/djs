// Plain dumps (0207): a continuation's prefix and the dump's callbacks in order. pjz's
// fuzzer found these: a `for await` over an async generator whose continuation the
// callback makes after the loop's continuation read it, an accessor pair after the site
// of an object literal, a callback that needs a class's instance before the loop's block
// defines the continuation it records, and a `for...of` over a suspended sync generator.
async function* g() { yield await tools.echo(1); }
for await (const x of g()) console.log(x, await tools.echo(x));

const o = { a: await tools.echo(1), get b() { return 2; }, set b(v) {} };
console.log(JSON.stringify(o), Object.getOwnPropertyDescriptor(o, "b").get !== undefined);

class C { m() { return tools; } }
const c = new C();
for (let i = 0; i < 2; i++) {
  const h = async () => console.log(`${await tools.echo("ok")}-${i}`);
  await h();
}
console.log(c.m() === tools);

let n = 1;
function* s(k) { for (let i = 0; i < k; i++) yield i + n; }
for (const x of s(2)) n += await tools.echo(x);
console.log(n);
