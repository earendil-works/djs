// Imported from dquickts tests/test_typescript.ts (test_enums) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// difftest: skip-djs dynamic code (eval/Function) is a djs non-goal

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

function test_enums() {
    enum E { A, B, C = 10, D, E = "e", F = D + 1 }
    assert(E.A + E.B + E.C + E.D, 22);
    assert(E[0] + E[11], "AD");
    assert(E.E, "e");
    assert(E["e"], undefined, "no reverse mapping for string members");
    assert(E.F, 12);
    assert(Object.keys(E).join(), "0,1,10,11,12,A,B,C,D,E,F");

    /* computed members and references to earlier members */
    let count = 0;
    function next() { return ++count * 100; }
    enum C { X = next(), Y = X + 1, Z = "abc".length, W = Z * 2 }
    assert(C.X, 100);
    assert(C.Y, 101);
    assert(C[101], "Y");
    assert(C.W, 6);

    /* merged declarations */
    enum M { A = 1 }
    enum M { B = A + 1, C = B * 2 }
    assert(M.A + M.B + M.C, 7);
    assert(M[4], "C");

    /* names: strings and keywords */
    enum N { "a-b" = 1, if = 2, default = N.if + 1 }
    assert(N["a-b"] + N.if + N.default, 6);

    /* constant expressions */
    enum K { A = 1 << 4, B = A | 3, C = ~B, D = -(2 ** 3), E = "x" + "y", F = 7 % 4, G = 100 >>> 2, H = (1 + 2) * 3 }
    assert(K.A + K.B + K.C + K.D, 16 + 19 - 20 - 8);
    assert(K.E + K.F + K.G + K.H, "xy3259");

    /* enum and namespace merge */
    enum Color { Red, Green }
    namespace Color { export function name(c: Color) { return Color[c]; } }
    assert(Color.name(Color.Green), "Green");

    /* block scoped */
    {
        enum Inner { A = 5 }
        assert(Inner.A, 5);
    }
    assert(eval("typeof Inner"), "undefined", "block scoped enum");
}

test_enums();
