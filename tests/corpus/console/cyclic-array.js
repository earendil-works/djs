// A cyclic array falls back to String(), i.e. join. QuickJS's join has no cycle
// detection and overflows the stack (V8 prints the cycle as an empty string).
const a = [1];
a.push(a);
console.log("before");
console.log(a);
