// Imported from dquickts tests/test_language.js (test_function_length) by
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

function test_function_length()
{
    assert( ((a, b = 1, c) => {}).length, 1);
    assert( (([a,b]) => {}).length, 1);
    assert( (({a,b}) => {}).length, 1);
    assert( ((c, [a,b] = 1, d) => {}).length, 1);
}

test_function_length();
