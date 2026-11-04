// Found by the plain-dump fuzzer (0208): an array pattern with a default after a return
// or throw (unreachable code) once failed to compile ("unreachable relative handler
// depth").
function f() {
  return 1;
  let [a = 1] = [];
}
function g() {
  throw new Error("g");
  const [b = 2, c = 3] = [];
}
console.log(f());
try {
  g();
} catch (e) {
  console.log(e.message);
}
