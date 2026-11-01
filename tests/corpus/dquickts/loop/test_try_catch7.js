// Imported from dquickts tests/test_loop.js (test_try_catch7) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

function assert(actual, expected, message) {
    if (arguments.length == 1)
        expected = true;

    if (actual === expected)
        return;

    if (actual !== null && expected !== null
    &&  typeof actual == 'object' && typeof expected == 'object'
    &&  actual.toString() === expected.toString())
        return;

    throw Error("assertion failed: got |" + actual + "|" +
                ", expected |" + expected + "|" +
                (message ? " (" + message + ")" : ""));
}

function test_try_catch7()
{
    var s;
    s = "";

    try {
        try {
            s += "t";
            throw "a";
        } finally {
            s += "f";
        }
    } catch(e) {
        s += e;
    } finally {
        s += "g";
    }
    assert(s, "tfag", "catch");
}

test_try_catch7();
