// Parameter properties assign `this.x`: a plain dump cannot blank their modifiers, so it
// stays TypeScript (0209). The class is used again after the load.
class P {
  constructor(public x: number, private readonly y: number = 2) {}
  sum(): number { return this.x + this.y; }
}
const p = new P(7);
await tools.echo(1);
const q = new P(1, 5);
console.log(p.x, p.sum(), q.x, q.sum());
return q.x;
