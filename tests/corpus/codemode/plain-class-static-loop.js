// A class declared in a loop whose static initializer reads the iteration's binding: the
// dump's class text must not run the initializer again where `i` is not in scope (0209).
for (const i of [0, 1]) {
  class C { static count = i; }
  console.log(await tools.echo(i), C.count);
}
return 1;
