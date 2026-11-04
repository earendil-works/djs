// TypeScript held across awaits (plain dumps write it as JavaScript, D2879): annotations,
// generics, interfaces and type aliases, `as`, `satisfies`, `!`, typed class fields and
// methods, an arrow with a return type, a type-only import-free module shape.
interface Item { path: string; size?: number }
type Pair<A, B> = [A, B];
class Box<T> {
  value: T;
  private label: string = "box";
  constructor(value: T) { this.value = value; }
  async twice(this: Box<T>, f: (v: T) => Promise<T>): Promise<T[]> {
    const a: T = await f(this.value);
    const b = (await f(a)) as T;
    return [a, b];
  }
}
const sizes: Map<string, number> = new Map();
async function load<T extends Item>(items: readonly T[]): Promise<Pair<number, string[]>> {
  const names: string[] = [];
  for (const it of items) {
    const r = (await tools.echo({ path: it.path, size: it.size ?? 0 })) satisfies object;
    sizes.set(r.path!, r.size as number);
    names.push(<string>r.path);
  }
  return [names.length, names];
}
const [n, names]: Pair<number, string[]> = await load([{ path: "a" }, { path: "bb", size: 2 }]);
console.log(n, names.join(), [...sizes.entries()].join(";"));
const box = new Box<number>(3);
const got = await box.twice(async (v: number): Promise<number> => (await tools.echo(v * 2)) as number);
console.log(got.join());
const pick = <K extends keyof Item>(o: Item, k: K): Item[K] => o[k];
console.log(pick({ path: "p" }, "path"), await tools.echo("end" as const));
return { n, got };
