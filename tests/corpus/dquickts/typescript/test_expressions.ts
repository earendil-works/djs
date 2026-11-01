// Imported from dquickts tests/test_typescript.ts (test_expressions) by
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

function test_expressions() {
    function id<T>(x: T): T { return x; }
    const a = 1, b = 2, c = 3;

    /* generic call vs comparison */
    assert(id<number>(4), 4);
    assert(a < b > c, false, "(a < b) > c");
    assert(a < b >= c, false, "(a < b) >= c");
    assert(a < (b > c), false);
    const instantiated = id<string>;
    assert(instantiated("s"), "s");
    assert(new Map<string, number>([["k", 1]]).get("k"), 1);
    assert(new Array<number>().length, 0);
    const tag = (s: TemplateStringsArray, ...v: number[]) => s.raw[0] + v.length;
    assert(tag<number>`x${1}`, "x1");
    const nested = new Map<string, Array<number>>();
    assert(nested.size, 0);

    /* as, satisfies, non-null, angle bracket assertions */
    assert((1 + 2 as number) * 2, 6, "'as' binds tighter than '*' operands");
    assert(a as number < b, true, "'as' then '<'");
    assert(b as number * 3 + 1, 7, "(b as number) * 3 + 1");
    assert(b as number ** 3 ** 2, 512, "(b as number) ** (3 ** 2)");
    assert(1 + b satisfies number ** 2 - 1, 8, "((1 + b) satisfies number) ** 2 - 1");
    const cfg = { port: 80 } satisfies { port: number };
    assert(cfg.port, 80);
    const arr = [1, 2] as const;
    assert(arr.length, 2);
    const o: { p?: { q: number } } = { p: { q: 5 } };
    assert(o.p!.q, 5);
    assert(o.p!["q"]!, 5);
    let v = <number><unknown>"abc";
    assert(v, "abc");
    let x = 1;
    (x as number) = 2;
    (<number>x)++;
    x! += 1;
    assert(x, 4);
    const u = { k: 1 } as unknown as { k: number };
    assert(u.k, 1);

    /* arrow return types, in conditionals */
    const t = true;
    const f1 = (n: number): number => n + 1;
    const f2 = async (n: number): Promise<number> => n;
    const f3 = (n: number): { r: number } => ({ r: n });
    const f4 = (): (() => number) => () => 7;
    assert(f1(1) + f3(2).r + f4()(), 11);
    assert(typeof f2, "function");
    let y = 10, z = 20;
    /* 'x ? (y) : z => w' is a conditional: the arrow body is not followed by ':' */
    const c1 = t ? (y) : z => z;
    assert(c1, 10);
    const c2 = !t ? (y) : (z) => z;
    assert(typeof c2, "function");
    /* here the arrow has a return type */
    const c3 = t ? (n: number): number => n * 2 : null;
    assert(c3!(4), 8);
    const c4 = t ? (n): string => "r" + n : (n) => n;
    assert(c4(1), "r1");
    const c5 = t ? (a) : (b);
    assert(c5, 1);
}

test_expressions();
