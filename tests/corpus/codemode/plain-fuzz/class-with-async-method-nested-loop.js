// Found by the plain-dump fuzzer (0211): a class with an async method declared in an
// inner loop's body and naming the iteration's variable, dumped in a later iteration
// (the first iteration's class is still reachable).
for (const i of [0, 1]) {
  for (let j = 0; j < 1; j++) {
    let n = i;
    class C {
      get(x) { return x + n; }
      async fetch(x) { return await tools.echo(x); }
    }
    const o = new C(i > 0 ? await tools.echo(4) : 4);
    console.log(o.get(1));
  }
}
return "done";
