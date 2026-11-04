// Found by the plain-dump fuzzer (0211): a class in a loop body whose static field
// reads the iteration's variable.
for (const i of [0, 1]) {
  class C {
    static count = i;
  }
  console.log(await tools.echo(i), C.count);
}
return "done";
