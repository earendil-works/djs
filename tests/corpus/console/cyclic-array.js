// difftest: djs-expect InternalError: stack overflow => RangeError: Maximum call stack size exceeded
// A cyclic array falls back to String(), i.e. join. QuickJS's join has no cycle
// detection and overflows the stack (V8 prints the cycle as an empty string).
// difftest: no-stress recurses to a stack overflow: over 120 s in each stress mode (0053)
const a = [1];
a.push(a);
console.log("before");
console.log(a);
