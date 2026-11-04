// Accessors that capture the body's variables, live across awaits: a plain dump makes
// them in the continuation, and the object they sit on gets them by a patch (0209).
let n = 0;
const o = { get a() { n++; return n; } };
const box = { v: 1 };
Object.defineProperty(box, "w", { get() { return n * 10; }, set(x) { n = x; }, enumerable: false });
await tools.echo(1);
console.log(o.a, o.a, box.w);
box.w = 5;
await tools.echo(2);
console.log(o.a, box.w, Object.keys(box).join(","));
return o.a;
