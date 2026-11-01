// Imported from dquickts tests/test_typescript.ts (test_namespaces) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// 1 assertSyntaxError checks moved to test_namespaces_syntax_N cases.

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

function test_namespaces() {
    namespace A.B.C { export const v = 1; }
    assert(A.B.C.v, 1);

    /* exported variables are properties of the namespace object, also
       for the code of the namespace */
    namespace Counter {
        export let count = 0;
        export function inc() { return ++count; }
        export function get() { return count; }
        let hidden = 5;
        export const [first, second] = [hidden, hidden + 1];
        export const { prop } = { prop: "p" };
    }
    Counter.inc();
    Counter.count += 10;
    assert(Counter.get(), 11);
    assert(Counter.first + Counter.second, 11);
    assert(Counter.prop, "p");
    assert(Object.keys(Counter).includes("hidden"), false);

    /* merged blocks see the exports of the other blocks */
    namespace Merged { export const x = 1; }
    namespace Merged { export const y = x + 1; export function f() { return z; } }
    namespace Merged { export const z = y + 1; }
    assert(Merged.f(), 3);

    /* merging with functions and classes */
    function Fn() { return 1; }
    namespace Fn { export const extra = 2; }
    assert(Fn() + Fn.extra, 3);
    class Cls { static s = 1; }
    namespace Cls { export const t = 2; }
    assert(Cls.s + Cls.t, 3);

    /* nested namespaces, classes, enums and non-exported values */
    namespace Outer {
        const secret = 10;
        export namespace Inner { export const w = secret + 1; }
        export class K { v = Inner.w; }
        export enum E { A = Inner.w }
        export import Alias = Inner.w;
    }
    assert(Outer.Inner.w + new Outer.K().v + Outer.E.A + Outer.Alias, 44);
    assert(Outer.E[11], "A");

    /* type-only namespaces emit nothing */
    namespace Types { export interface I {} export type T = number; namespace Deep { export type U = string; } }
    assert(typeof Types, "undefined");

    /* 'export declare' members exist at runtime (assigned from outside) */
    namespace Ambient { export declare let late: number; export function read() { return late; } }
    (Ambient as unknown as { late: number }).late = 8;
    assert(Ambient.read(), 8);

    /* import aliases */
    import Alias = Outer.Inner;
    import TypeAlias = Types.T;
    assert(Alias.w, 11);

    /* 'this' and 'return' */
}

test_namespaces();
