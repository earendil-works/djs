// Decorators and auto-accessors with TypeScript (0065): modifiers, annotations, generics,
// members without code (their decorators are evaluated, not applied).
const log: string[] = [];
function dec(n: string) { return (v: unknown, ctx: ClassMemberDecoratorContext | ClassDecoratorContext) => { log.push(n + ":" + ctx.kind + ":" + String(ctx.name)); }; }

@dec("A") class A<T> implements Iterable<T> {
  @dec("x") public x: number = 1;
  @dec("y") private readonly y?: string;
  @dec("acc") accessor acc: number = 2;
  @dec("m") protected m<U>(u: U): U { return u; }
  @dec("abs") n(): void;
  @dec("decl") declare d: number;
  @dec("s") static accessor s: string = "s";
  [Symbol.iterator](): Iterator<T> { return [][Symbol.iterator](); }
}
class B extends A<number> { n() {} }
const b = new B();
console.log(log.join(), b.x, b.acc, (A as any).s);
