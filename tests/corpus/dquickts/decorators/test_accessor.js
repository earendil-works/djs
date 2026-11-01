// Imported from dquickts tests/test_decorators.js (test_accessor) by
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

function test_accessor() {
    const sym = Symbol("s");
    const key = "k";
    class C {
        accessor a;
        accessor b = 1;
        accessor "c" = 2;
        accessor [key] = 3;
        accessor [sym] = 4;
        accessor 5 = 5;
        static accessor s = 6;
        accessor #p = 7;
        static accessor #sp = 8;
        getP() { return this.#p; }
        setP(v) { this.#p = v; }
        static getSP() { return C.#sp; }
        static hasP(o) { return #p in o; }
    }
    const c = new C();
    assert(c.a, undefined);
    assert(c.b + c.c + c.k + c[sym] + c[5] + C.s, 21);
    c.b = 10;
    assert(c.b, 10);
    assert(Object.hasOwn(c, "b"), false, "the storage is private");
    const desc = Object.getOwnPropertyDescriptor(C.prototype, "b");
    assert(typeof desc.get + typeof desc.set, "functionfunction");
    assert(desc.enumerable, false);
    assert(desc.get.name + "," + desc.set.name, "get b,set b");
    assert(c.getP(), 7);
    c.setP(70);
    assert(c.getP(), 70);
    assert(C.getSP(), 8);
    assert(C.hasP(c), true);
    assert(C.hasP({}), false);
    /* the getter checks the instance */
    assertThrows(TypeError, () => desc.get.call({}));
    /* 'accessor' is still an identifier */
    class D {
        accessor;
        accessor
        x = 1;
        accessor() { return 2; }
    }
    assert(new D().x, 1);
    assert(Object.hasOwn(new D(), "accessor"), true);
    /* duplicate private names */
    assertThrows(SyntaxError, () => eval("class E { accessor #x; #x; }"));
    assertThrows(SyntaxError, () => eval("class E { accessor #x; get #x() {} }"));
    assertThrows(SyntaxError, () => eval("class E { accessor m() {} }"));
}

test_accessor();
