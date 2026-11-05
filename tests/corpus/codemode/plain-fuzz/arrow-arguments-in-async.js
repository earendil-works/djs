// Found by the plain-dump fuzzer (0211): an arrow function capturing `arguments` of
// an async function that waits.
async function g() {
  const f = () => arguments.length + ":" + [...arguments].join(",");
  await tools.echo(1);
  return f();
}
console.log(await g(1, 2));
return "done";
