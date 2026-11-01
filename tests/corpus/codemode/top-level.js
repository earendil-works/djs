// The script is an async function body: var, let, functions and this.
var v = 1;
let l = 2;
function hoisted() { return "hoisted"; }
console.log(v, l, hoisted(), typeof this, typeof globalThis.v);
console.log(typeof arguments);
if (v) return "early";
console.log("not reached");
