// Imported from dquickts tests/test_decorators.js (test_replacement) by
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

function test_replacement() {
    const wrap = (m, ctx) => function (...args) { return "<" + m.call(this, ...args) + ">"; };
    const twice = (m, ctx) => function (v) { return m.call(this, v * 2); };
    class C {
        @wrap m(x) { return x; }
        @wrap static sm() { return "s"; }
        @wrap get g() { return "g"; }
        set s(v) { this.v = v; }
        @twice set s(v) { this.v = v; }
        @wrap #pm() { return "pm"; }
        @wrap get #pg() { return "pg"; }
        @twice set #ps(v) { this.pv = v; }
        @wrap *gen() { yield 1; }
        test() {
            this.#ps = 2;
            return [this.#pm(), this.#pg, this.pv, #pm in this, this.#pm === this.#pm].join();
        }
    }
    const c = new C();
    assert(c.m(1), "<1>");
    assert(C.sm(), "<s>");
    assert(c.g, "<g>");
    c.s = 2;
    assert(c.v, 4);
    assert(c.test(), "<pm>,<pg>,4,true,true");
    assertThrows(TypeError, () => C.prototype.test.call({}));
    /* auto-accessor: get, set and init */
    const acc = ({ get, set }, ctx) => ({
        get() { return get.call(this) * 10; },
        set(v) { set.call(this, v + 1); },
        init(v) { return v + 100; },
    });
    class D {
        @acc accessor a = 1;
        @acc accessor #b = 2;
        @acc static accessor c = 3;
        getB() { return this.#b; }
        setB(v) { this.#b = v; }
    }
    const d = new D();
    assert(d.a, 1010);
    d.a = 1;
    assert(d.a, 20);
    assert(d.getB(), 1020);
    d.setB(5);
    assert(d.getB(), 60);
    assert(D.c, 1030);
    /* class replacement */
    let orig;
    const replace = (cls, ctx) => {
        orig = cls;
        return class extends cls { static replaced = true; };
    };
    @replace class E {
        static self = E;
        static that = this;
        static getE() { return E; }
    }
    assert(E.replaced, true);
    assert(Object.getPrototypeOf(E), orig);
    assert(E.getE(), E, "the class binding is the new class");
    assert(E.self, E);
    assert(E.that, E, "the static fields are defined on the new class");
    const F = @replace class {};
    assert(F.replaced && orig.name === "F", true);
}

test_replacement();
