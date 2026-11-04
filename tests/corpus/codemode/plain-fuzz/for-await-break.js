// Found by the plain-dump fuzzer (0208): `for await` over an async generator suspended at
// its await, the loop body leaving with `break` (the loop's scaffold closes the iterator).
async function* g(k) { for (let i = 0; i < k; i++) yield await tools.echo(i); }
for await (const x of g(2)) { console.log(x); break; }
console.log("after");
async function* h() { yield await tools.echo(1); yield 2; }
for await (const x of h()) { console.log(x, await tools.echo(x)); break; }
console.log("done");
