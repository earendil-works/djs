// Imported from dquickts tests/test_language.js (test_parse_arrow_function) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// difftest: skip-djs dynamic code (eval/Function) is a djs non-goal

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

function test_parse_arrow_function()
{
    assert(typeof eval("() => {}\n() => {}"), "function");
    assert(eval("() => {}\n+1"), 1);
    assert(typeof eval("x => {}\n() => {}"), "function");
    assert(typeof eval("async () => {}\n() => {}"), "function");
    assert(typeof eval("async x => {}\n() => {}"), "function");
}

test_parse_arrow_function();
