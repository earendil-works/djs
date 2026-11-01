// Imported from dquickts tests/test_decorators.js (test_misc) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// difftest: skip-djs direct eval: access to the caller's scope is a non-goal (D014)

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

function test_misc() {
    /* named evaluation of anonymous classes */
    let name;
    const getName = (cls, ctx) => { name = ctx.name + "/" + cls.name; };
    const X = @getName class {};
    assert(name, "X/X");
    const obj = { Y: @getName class {} };
    assert(name, "Y/Y");
    @getName class Z {}
    assert(name, "Z/Z");
    /* 'this' and decorators in methods and nested classes */
    const o = {
        tag: "o",
        make() {
            const tag = (v, ctx) => { ctx.addInitializer(function () { this.tag = o.tag; }); };
            return class { @tag m() {} };
        },
    };
    const K = o.make();
    assert(new K().tag, "o");
    class Outer {
        static #count = 0;
        static inc = (v, ctx) => { Outer.#count++; };
        static make() {
            return class { @Outer.inc m() {} @Outer.inc n() {} };
        }
        static get count() { return Outer.#count; }
    }
    Outer.make();
    assert(Outer.count, 2);
    /* decorated private methods with direct eval */
    const w = (m) => function () { return "w" + m.call(this); };
    class P {
        @w #m() { return 1; }
        run(src) { return eval(src); }
    }
    assert(new P().run("this.#m()"), "w1");
    /* generators and async methods keep their kind */
    const id = (m) => m;
    class G {
        @id *gen() { yield 1; yield 2; }
        @id async am() { return 3; }
    }
    assert([...new G().gen()].join(), "1,2");
    assert(new G().am() instanceof Promise, true);
}

test_misc();
