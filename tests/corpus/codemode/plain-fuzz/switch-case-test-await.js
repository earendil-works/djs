// Found by the plain-dump fuzzer (0211): an await in a case test, then one in a later
// case's body reached by falling through, dumped again there.
switch (0) {
  case await tools.echo(5):
    console.log("five");
  case 0:
    console.log(await tools.echo("x"));
}
return "done";
