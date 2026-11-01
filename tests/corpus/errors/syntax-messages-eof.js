// difftest: stacks
// SyntaxError messages at the end of input and for Function bodies that close the
// function early, as qjs words them (0108, D2071).
const indirect = eval;
const sources = ["1 +", "(", "[", "{ a", "a ?", "x =>", "let x =", "if (", "for (", "new",
  "a ? b :", "import(", "({a:", "`${", "x => {", "1 + 2 +\n", "a,", "throw", "function f() {",
  "try {} catch (e) { 1", "switch (1) { case 1:", "switch (1) { default: a", "class A {",
  "class A { m() {}", "(function () {", "label: {", "do {", "({ get a() {", "class A { static { 1"];
for (const src of sources) {
  try { indirect(src); console.log("no error", JSON.stringify(src)); } catch (e) {
    console.log(e.name + ": " + e.message, JSON.stringify(src));
    console.log(e.stack.split("\n")[0]);
  }
}
const argLists = [["x", "}"], ["}"], ["x", "}{"], ["a)", "b"], ["", "})"], ["x", "return 1; }"],
  ["a = 1)", ""], ["", "} function g() {"], ["){}; x = 1; (function(", ""], ["a) { return 1", "2"]];
for (const args of argLists) {
  try { Function(...args); console.log("no error", JSON.stringify(args)); } catch (e) {
    console.log(e.name + ": " + e.message, JSON.stringify(args));
    console.log(e.stack.split("\n")[0]);
  }
}
