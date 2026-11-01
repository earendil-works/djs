// Imported from dquickts tests/test_decorators.js (test_metadata) by
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

function test_metadata() {
    const meta = (k, v) => (_, ctx) => { ctx.metadata[k] = v; };
    @meta("a", 1) class A {
        @meta("m", 2) m() {}
    }
    class B extends A {
        @meta("b", 3) #f;
    }
    class C extends B {}
    class D {}
    const ma = A[Symbol.metadata], mb = B[Symbol.metadata];
    assert(Object.getPrototypeOf(ma), null);
    assert(ma.a + ma.m, 3);
    assert(Object.getPrototypeOf(mb), ma);
    assert(mb.b + mb.a, 4);
    assert(Object.hasOwn(mb, "a"), false);
    assert(C[Symbol.metadata], mb, "inherited through the class");
    assert(D[Symbol.metadata], undefined);
    assert(typeof Symbol.metadata, "symbol");
    class E extends null { @meta("e", 1) static m() {} }
    assert(Object.getPrototypeOf(E[Symbol.metadata]), null);
}

test_metadata();
