// Imported from dquickts tests/test_typescript.ts (test_type_grammar) by
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

/* the type grammar: the declarations must be skipped entirely */
function test_type_grammar() {
    type Prim = any | unknown | never | void | null | undefined | object | number | bigint | string | boolean | symbol;
    type Lit = 1 | -1 | 1n | -2n | "s" | `t` | true | false;
    type Tpl<T extends string> = `a${T}b${Uppercase<T>}c`;
    type Fn = (a: number, b?: string, ...c: number[]) => void;
    type GenFn = <T>(x: T) => T;
    type Ctor = new <T>(x: T) => object;
    type AbsCtor = abstract new () => object;
    type Obj = { a: number; b?: string; readonly c: boolean; -readonly [K in "d"]-?: number };
    type Mapped<T> = { +readonly [K in keyof T as `get${Capitalize<string & K>}`]+?: () => T[K] };
    type Cond<T> = T extends (infer U extends string)[] ? U : T extends Promise<infer V> ? V : never;
    type Indexed = Obj["a"] | Obj[keyof Obj] | string[][number];
    type Tuple = [first: number, second?: string, ...rest: boolean[]] | readonly [number, string];
    type Q = typeof globalThis.Math.PI | keyof typeof Math;
    /* import types: covered here for the type skipper only */
    type Imp = typeof import("./assert.js") | import("./assert.js").Foo<number>;
    type Pred = (x: unknown) => x is string;
    type Asserts = (x: unknown) => asserts x is string;
    type ThisPred = { isFoo(): this is Obj };
    type Nested = Array<Array<Map<string, Set<number>>>>;
    type Unique = { readonly k: unique symbol };
    type Parens = (string | number)[] | (() => void) | ((x: number) => string)[];
    type Leading = | "a" | "b";
    type LeadingAnd = & { a: 1 } & { b: 2 };
    type Accessors = { get x(): number; set x(v: number) };
    let prims: Prim[] = [], lits: Lit[] = [], t: Tpl<"x"> | undefined, f: Fn | GenFn | Ctor | AbsCtor | undefined;
    let n: Nested = [[new Map()]];
    assert(n[0].length + prims.length + lits.length, 1);
}

test_type_grammar();
