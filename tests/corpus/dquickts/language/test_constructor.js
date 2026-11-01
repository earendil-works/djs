// Imported from dquickts tests/test_language.js (test_constructor) by
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

function test_constructor()
{
    function *G() {}
    let ex
    try { new G() } catch (ex_) { ex = ex_ }
    assert(ex instanceof TypeError)
    assert(ex.message, "G is not a constructor")
}

test_constructor();
