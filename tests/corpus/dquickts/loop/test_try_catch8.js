// Imported from dquickts tests/test_loop.js (test_try_catch8) by
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

function test_try_catch8()
{
    var i, s;

    s = "";
    for(var i in {x:1, y:2}) {
        try {
            s += i;
            throw "a";
        } catch (e) {
            s += e;
        } finally {
            s += "f";
        }
    }
    assert(s === "xafyaf");
}

test_try_catch8();
