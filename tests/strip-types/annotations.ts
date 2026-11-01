// Annotations, generics, assertions.
let a: number = 1, b!: string, c: Array<Map<string, number[]>> = [];
const d: { x: number; y?: string } /* kept */ = { x: 1 };
function f<T extends object = {}>(this: Window, x: T, y?: number, ...rest: string[]): T {
  return x;
}
function g(this: unknown) {}
const h = <T,>(x: T): T => x;
const i = async <T>(x: T): Promise<T> => x;
const j = (x: number): { r: number } => ({ r: x });
const k = a as unknown as number;
const l = d.x! + (d.y as string).length;
const m = f<{ a: 1 }>({ a: 1 }) satisfies object;
const n = new Map<string, number>();
const o = f<number[]>`tpl`;
const p = f<string>;
let q = a /* c1 */ as /* c2 */ number;
let r: typeof a, s: keyof typeof d, t: `x${number}`;
let u: (a: number) => void, v: new () => object, w: [a: number, b?: string];
for (let z: number = 0; z < 1; z++) {}
try {} catch (e: unknown) {}
const { x: dx }: { x: number } = d;
function asserts(x: unknown): asserts x is number {}
function pred(x: unknown): x is string { return true; }
