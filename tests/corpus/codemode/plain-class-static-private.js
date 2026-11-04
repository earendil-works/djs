// A static private field whose initializer is not a literal: only the class's code can set
// it, and evaluating the class again would run the initializer again, so plain dumps
// refuse it (0209).
let made = 0;
class C {
  static #id = ++made;
  static id() { return C.#id; }
}
await tools.echo(1);
console.log(C.id(), made);
return made;
