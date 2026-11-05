// Found by the plain-dump fuzzer (0207, seed 276): a non-null assertion ending an array
// element (`n1!]`). The continuation is written from the TypeScript original, the origin
// read back from the dump is JavaScript with the `!` blanked, where the element's node
// ends before the blank: a re-dump must still find the continuation's layout.
let n1 = 0;
async function f3(a4: number): Promise<number> {
  console.log("p5", JSON.stringify([(await tools.echo(a4)), n1!]));
  return (await tools.echo((await tools.echo(2))));
}
console.log("p6", JSON.stringify((await f3((1 as number)))));
