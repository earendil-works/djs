// Errors print as "Name: message" (the stack frames that follow are not compared).
console.log(new Error("plain"));
console.log(new TypeError("typed"));
console.log(new RangeError(""));
console.log(new Error());
console.log("prefix", new SyntaxError("in the middle"), "suffix");
class MyError extends Error {
  constructor(msg) {
    super(msg);
    this.name = "MyError";
  }
}
console.log(new MyError("custom"));
const e = new Error("renamed");
e.name = "Renamed";
console.log(e);
console.log(new Error("with cause", { cause: "because" }));
console.log(new AggregateError([new Error("inner")], "aggregate"));
// Inside JSON an error is an object with its own enumerable properties only.
console.log({ err: new Error("hidden") }, [new TypeError("x")]);
const withProp = new Error("p");
withProp.code = 42;
console.log([withProp]);
console.log(new Error("multi\nline message"));
