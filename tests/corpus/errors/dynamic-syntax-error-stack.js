// difftest: stacks
// A SyntaxError from indirect eval or a dynamic function constructor starts its stack
// with qjs's `<input>:line:col` frame (0106), counted against Error.stackTraceLimit.
// Messages that differ from qjs's are not printed here (see the issue's follow-ups).
const indirect = eval;
const sources = ["(", "let x = ;\nfoo", "a b", "\n\n  )", "{", "x = 1 +* 2"];
for (const src of sources) {
  try { indirect(src); } catch (e) { console.log(e.name, JSON.stringify(src)); console.log(e.stack); }
}
const argLists = [["("], ["a", "b", ")"], ["return 1 +"], ["a b"], ["a,", "b c", "return 1"], ["x", "1 +* 2"]];
for (const args of argLists) {
  try { Function(...args); } catch (e) { console.log(e.name, JSON.stringify(args)); console.log(e.stack); }
  try { new Function(...args); } catch (e) { console.log(e.name); console.log(e.stack); }
}
const AsyncFunction = (async function () {}).constructor;
const GeneratorFunction = (function* () {}).constructor;
for (const C of [AsyncFunction, GeneratorFunction]) {
  try { C("a b"); } catch (e) { console.log(e.name); console.log(e.stack); }
}
function viaFunction() { return indirect("("); }
try { viaFunction(); } catch (e) { console.log(e.stack); }
try { indirect("indirect('a b')"); } catch (e) { console.log(e.stack); }
try { JSON.parse("{"); } catch (e) { console.log(e.name, e.message); console.log(e.stack); }
Error.stackTraceLimit = 1;
try { Function("("); } catch (e) { console.log(e.stack); }
Error.stackTraceLimit = 0;
try { indirect("("); } catch (e) { console.log(JSON.stringify(e.stack)); }
Error.stackTraceLimit = 10;
