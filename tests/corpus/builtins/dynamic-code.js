// Indirect eval, the dynamic function constructors and queueMicrotask (0059, D014).
// Completion values of eval come with 0058; the eval cases here only use side effects.
const out = [];
const t = (label, f) => {
  try {
    out.push(label + " = " + String(f()));
  } catch (e) {
    out.push(label + " -> " + e.name + ": " + e.message);
  }
};
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
const GeneratorFunction = Object.getPrototypeOf(function* () {}).constructor;
const AsyncGeneratorFunction = Object.getPrototypeOf(async function* () {}).constructor;

t("Function", () => Function("a", "b", "return a + b")(2, 3));
t("new Function", () => new Function("a, b", "c", "return a * b * c")(2, 3, 4));
t("no args", () => Function()());
t("toString", () => Function("a", "b", "return a").toString());
t("toString empty", () => Function().toString());
t("name/length", () => [Function("a,b", "").name, Function("a,b", "").length, Function.length].join());
t("prototype", () => typeof Function("").prototype + " " + (Object.getPrototypeOf(Function("")) === Function.prototype));
t("global scope", () => { globalThis.dynX = 7; return Function("return dynX")(); });
t("syntax error", () => Function("return )"));
t("bad params", () => Function("a b", ""));
t("ToString order", () => {
  const seen = [];
  Function({ toString() { seen.push("p1"); return "a"; } }, { toString() { seen.push("body"); return ""; } });
  return seen.join();
});
t("generator", () => [...GeneratorFunction("a", "yield a; yield a * 2")(5)].join());
t("generator toString", () => GeneratorFunction("yield 1").toString());
t("generator proto", () => Object.getPrototypeOf(GeneratorFunction("")) === GeneratorFunction.prototype);
t("async toString", () => AsyncFunction("x", "return await x").toString());
t("async generator toString", () => AsyncGeneratorFunction("yield 1").toString());
t("subclass", () => {
  class F extends Function {}
  const f = new F("return 1");
  return [f(), f instanceof F, Object.getPrototypeOf(f) === F.prototype].join();
});
t("indirect eval side effect", () => { (0, eval)("globalThis.dynY = 3 * 4"); return dynY; });
t("eval syntax error", () => (0, eval)("1 + ;"));
t("eval non-string", () => { const o = {}; return (0, eval)(o) === o; });
t("eval name/length", () => eval.name + " " + eval.length);
t("queueMicrotask arity", () => queueMicrotask.length + " " + queueMicrotask.name);
t("queueMicrotask non-callable", () => queueMicrotask(1));

const order = [];
queueMicrotask(() => order.push("micro 1"));
Promise.resolve().then(() => order.push("then"));
queueMicrotask(() => order.push("micro 2"));
await AsyncFunction("x", "return await x")(9).then((v) => order.push("async " + v));
await null;
out.push("order = " + order.join(", "));
console.log(out.join("\n"));
