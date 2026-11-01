// Imported from dquickts tests/test_language.js (test_optional_chaining) by
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

/* optional chaining tests not present in test262 */
function test_optional_chaining()
{
    var a, z;
    z = null;
    a = { b: { c: 2 } };
    assert(delete z?.b.c, true);
    assert(delete a?.b.c, true);
    assert(JSON.stringify(a), '{"b":{}}', "optional chaining delete");

    a = { b: { c: 2 } };
    assert(delete z?.b["c"], true);
    assert(delete a?.b["c"], true);
    assert(JSON.stringify(a), '{"b":{}}');

    a = {
        b() { return this._b; },
        _b: { c: 42 }
    };

    assert((a?.b)().c, 42);

    assert((a?.["b"])().c, 42);
}

test_optional_chaining();
