// Imported from dquickts tests/test_language.js (test_prototype) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

function assert(actual, expected, message) {
    if (arguments.length == 1)
        expected = true;

    if (Object.is(actual, expected))
        return;

    if (actual !== null && expected !== null
    &&  typeof actual == 'object' && typeof expected == 'object'
    &&  actual.toString() === expected.toString())
        return;

    throw Error("assertion failed: got |" + actual + "|" +
                ", expected |" + expected + "|" +
                (message ? " (" + message + ")" : ""));
}

function test_prototype()
{
    var f = function f() { };
    assert(f.prototype.constructor, f, "prototype");

    var g = function g() { };
    /* QuickJS bug */
    Object.defineProperty(g, "prototype", { writable: false });
    assert(g.prototype.constructor, g, "prototype");
}

test_prototype();
