// A generator suspended across a dump keeps its function's prototype (0216's finding,
// D2915): the default one, one with a property added, one replaced; and a function's
// default prototype held elsewhere is the function's again.
function* plain() { yield 1; yield 2; }
function* counter() { let i = 0; while (true) yield i++; }
counter.prototype.label = "c";
function* replaced() { yield "r"; }
replaced.prototype = { __proto__: Object.getPrototypeOf(plain.prototype), kind: "new" };
function F() {}
const p = plain(), c = counter(), r = replaced(), fp = F.prototype;
p.next(); c.next();
await tools.echo(1);
console.log(Object.getPrototypeOf(p) === plain.prototype, Object.getPrototypeOf(c) === counter.prototype, c.label);
console.log(Object.getPrototypeOf(r) === replaced.prototype, r.kind, fp === F.prototype, fp.constructor === F);
console.log(p.next().value, c.next().value, r.next().value);
