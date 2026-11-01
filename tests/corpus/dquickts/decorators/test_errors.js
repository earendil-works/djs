// Imported from dquickts tests/test_decorators.js (test_errors) by
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

function test_errors() {
    const ret = (v) => () => v;
    assertThrows(TypeError, () => { class C { @(ret(1)) m() {} } });
    assertThrows(TypeError, () => { class C { @(ret({})) f; } });
    assertThrows(TypeError, () => { class C { @(ret(1)) accessor a; } });
    assertThrows(TypeError, () => { class C { @(ret({ get: 1 })) accessor a; } });
    assertThrows(TypeError, () => { @(ret(null)) class C {} });
    let saved;
    class D { @((v, ctx) => { saved = ctx; }) m() {} }
    assertThrows(TypeError, () => saved.addInitializer(() => {}));
    assertThrows(TypeError, () => { class C { @((v, ctx) => { ctx.addInitializer(1); }) m() {} } });
    /* exceptions from the decorators */
    const thrower = () => { throw new RangeError(); };
    assertThrows(RangeError, () => { @thrower class C {} });
    assertThrows(RangeError, () => { class C { @thrower static m() {} } });
    /* syntax */
    assertThrows(SyntaxError, () => eval("class C { @dec static { } }"));
    assertThrows(SyntaxError, () => eval("class C { @dec constructor() {} }"));
    assertThrows(SyntaxError, () => eval("@dec function f() {}"));
    assertThrows(SyntaxError, () => eval("@dec let x;"));
    assertThrows(SyntaxError, () => eval("class C { @dec[0] m() {} }"));
    assertThrows(SyntaxError, () => eval("class C { m(@dec x) {} }"));
    /* decorators are evaluated in the class scope, the class binding is
       in TDZ */
    assertThrows(ReferenceError, () => { class C { @(C) m() {} } });
}

test_errors();
