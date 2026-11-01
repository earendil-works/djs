// Type-only statements, and the `;` that keeps the code around them apart.
let x = 1
type A = number
interface I { a: A }
declare const y: number
declare function df(): void
declare class DC { m(): void }
declare namespace DN { const z: number }
declare module "m" {}
declare global { interface W {} }
declare enum DE { A }
namespace TypesOnly { export type T = 1 }
function over(a: string): string;
function over(a: number): number;
function over(a: any) { return a }
x++
type B = A;
{
  type C = 1
}
function body() {
  type D = 1;
  return 1
}
if (x) {
  type E = 1
}
let last = x as number
[1].forEach(() => {})
let s2 = x satisfies number;
function ret() {
  return x as any
}
