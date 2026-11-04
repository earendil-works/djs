// difftest: plain-xfail the static field's initializer runs again at load where the loop variable is not in scope: ReferenceError
// Found by the plain-dump fuzzer (0211): a class in a loop body whose static field
// reads the iteration's variable.
for (const i of [0, 1]) {
  class C {
    static count = i;
  }
  console.log(await tools.echo(i), C.count);
}
return "done";
