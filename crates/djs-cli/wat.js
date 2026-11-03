// djs-dump 2: 3 bindings (c, CounterType, makeCounter).
// Load it with Vm::load_dump (djs REPL: .restore FILE), which passes in $dump.
// Builtins are referenced by name, never written. The host globals console, tools, print, later are re-bound by name.
// c sharing ty, initial
$dump.s1 = (() => {
  let ty = 0;
  let initial = 45;
  return { c: () => { if (ty === CounterType.Asc) { return ++initial } else { --initial } } };
})();
let c = $dump.s1.c;
var CounterType = { 0: "Asc", 1: "Desc", Asc: 0, Desc: 1 };
function makeCounter(ty, initial = 0) {
return () => { if (ty === CounterType.Asc) { return ++initial } else { --initial } };
}
// Links that close cycles, and attributes.
Object.defineProperty($dump.s1.c, "name", { value: "" });
