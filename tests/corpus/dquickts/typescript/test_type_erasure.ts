// Imported from dquickts tests/test_typescript.ts (test_type_erasure) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

function assert(actual, expected, message) {
    if (arguments.length === 1)
        expected = true;

    if (typeof actual === typeof expected) {
        if (Object.is(actual, expected))
            return;
        if (typeof actual === 'object') {
            if (actual !== null && expected !== null
            &&  actual.constructor === expected.constructor
            &&  actual.toString() === expected.toString())
                return;
        }
    }
    throw Error("assertion failed: got |" + actual + "|" +
                ", expected |" + expected + "|" +
                (message ? " (" + message + ")" : ""));
}

function assertThrows(err, func)
{
    var ex;
    ex = false;
    try {
        func();
    } catch(e) {
        ex = true;
        assert(e instanceof err);
    }
    assert(ex, true, "exception expected");
}

function test_type_erasure() {
    let a: number = 1;
    let b!: string;
    const c: readonly number[] = [1, 2];
    let d: [x: number, y?: string, ...rest: boolean[]] = [1];
    let fn: (x: number, ...r: string[]) => void = () => {};
    let ctor: abstract new (...args: unknown[]) => object;
    let sym: unique symbol;
    function add(x: number, y?: number, ...rest: number[]): number {
        return x + (y ?? 0) + rest.length;
    }
    function withThis(this: { k: number }, n: number): number {
        return this.k + n;
    }
    function defaults({ p, q }: { p: number; q?: number } = { p: 1 },
                      [r]: [number] = [2]): number {
        return p + (q ?? 0) + r;
    }
    const arrow = (x: number, y: string = "y"): string => x + y;
    const asyncArrow = async <T,>(x: T): Promise<T> => x;
    const generic = <T extends object = {}>(x: T): T => x;
    const constGeneric = <const T extends readonly unknown[]>(x: T): T => x;
    assert(a + add(1, 2, 3, 4) + c.length + d.length, 9);
    assert(b, undefined);
    assert(withThis.call({ k: 1 }, 2), 3);
    assert(withThis.length, 1, "'this' is not a parameter");
    assert(defaults(), 3);
    assert(defaults({ p: 1, q: 2 }, [3]), 6);
    assert(arrow(1), "1y");
    assert(typeof asyncArrow(1).then, "function");
    assert(generic({ v: 2 }).v, 2);
    assert(constGeneric([1, 2] as const).length, 2);
    assert(typeof fn + typeof ctor + typeof sym, "functionundefinedundefined");

    /* catch binding, for loops, object literal methods */
    try {
        throw 1;
    } catch (e: unknown) {
        assert(e, 1);
    }
    let sum = 0;
    for (let i: number = 0; i < 3; i++)
        sum += i;
    for (const [k, v] of [["x", 1]] as [string, number][])
        sum += v;
    assert(sum, 4);
    const obj = {
        m<T>(x: T): T { return x; },
        get g(): number { return 1; },
        set s(v: number) {},
        async am<T>(): Promise<void> {},
    };
    assert(obj.m(2) + obj.g, 3);

    /* type predicates and assertion functions */
    function isString(x: unknown): x is string { return typeof x === "string"; }
    function assertNum(x: unknown): asserts x is number {}
    function assertThis(this: object): asserts this {}
    assert(isString("s"), true);
    assertNum(1);

    /* type declarations emit nothing */
    type Alias<in out T, const U = T> = T | U;
    interface Iface<T> extends Object { a?: T; readonly b: number; [k: string]: unknown; m?(): void; new (x: number): Iface<T>; <U>(u: U): U }
    declare var declaredVar: number;
    declare let declaredLet: string, declaredLet2: number;
    declare const declaredConst: number;
    declare function declaredFn(x: number): void;
    declare class DeclaredClass<T> extends Object implements Iface<T> { m(): void; }
    declare enum DeclaredEnum { A }
    declare const enum DeclaredConstEnum { A }
    declare namespace DeclaredNS { const x: number; }
    declare module DeclaredModule { }
    declare global { interface GlobalIface {} }
    assert(typeof declaredVar + typeof DeclaredClass + typeof DeclaredEnum +
           typeof DeclaredNS, "undefinedundefinedundefinedundefined");
    assertThrows(ReferenceError, () => declaredConst);
}

test_type_erasure();
