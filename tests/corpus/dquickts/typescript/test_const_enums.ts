// Imported from dquickts tests/test_typescript.ts (test_const_enums) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// 2 assertSyntaxError checks moved to test_const_enums_syntax_N cases.

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

/* const enums: members are compile time constants */
function test_const_enums() {
    /* used before its declaration: resolved when the variables are */
    function early() { return Later.A; }

    const enum CE { A = 1, B = A * 10, S = "str", T = `t` }
    const enum Later { A = 5 }
    function useCE() { return CE.B + CE["A"]; }
    function useStr() { return CE.S + CE.T; }

    assert(useCE(), 11);
    assert(useStr(), "strt");
    assert(early(), 5);

    /* the enum object is still emitted (tsc's preserveConstEnums) */
    assert(JSON.stringify(CE), '{"1":"A","10":"B","A":1,"B":10,"S":"str","T":"t"}');

    /* folded: changing the object has no effect on the compiled code */
    let reads = 0;
    Object.defineProperty(CE, "B", { get() { reads++; return 99; } });
    Object.defineProperty(Later, "A", { get() { reads++; return 99; } });
    assert(useCE(), 11);
    assert(early(), 5);
    assert(reads, 0);
    assert(useCE.toString().includes("CE.B"), true, "the source is kept");

    /* dynamic accesses use the object */
    const key = "B";
    assert(CE[key], 99);
    assert(reads, 1);

    /* constant expressions only */

    /* references to other enums */
    const enum Other { X = CE.A + Later.A, Y = Other.X * 2 }
    assert(Other.Y, 12);

    /* shadowing bindings are not folded */
    function shadowParam(CE: { B: number }) { return CE.B; }
    assert(shadowParam({ B: 3 }), 3);
    function shadowVar() { var CE = { B: 4 }; return CE.B; }
    assert(shadowVar(), 4);
    {
        let CE = { A: 6 };
        assert(CE.A, 6);
    }

    /* in a namespace */
    namespace NS { export const enum K { X = 7 } export const viaMember = K.X; }
    function useNS() { return NS.K.X; }
    Object.defineProperty(NS.K, "X", { get() { return 0; } });
    assert(useNS() + NS.viaMember, 14);
}

test_const_enums();
