// Imported from dquickts tests/test_builtin.js (test_number) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

var status = 0;

var throw_errors = true;

function throw_error(msg) {
    if (throw_errors)
        throw Error(msg);
    console.log(msg);
    status = 1;
}

function assert(actual, expected, message) {
    function get_full_type(o) {
        var type = typeof(o);
        if (type === 'object') {
            if (o === null)
                return 'null';
            if (o.constructor && o.constructor.name)
                return o.constructor.name;
        }
        return type;
    }

    if (arguments.length == 1)
        expected = true;

    if (typeof actual === typeof expected) {
        if (actual === expected) {
            if (actual !== 0 || (1 / actual) === (1 / expected))
                return;
        }
        if (typeof actual === 'number') {
            if (isNaN(actual) && isNaN(expected))
                return true;
        }
        if (typeof actual === 'object') {
            if (actual !== null && expected !== null
            &&  actual.constructor === expected.constructor
            &&  actual.toString() === expected.toString())
                return;
        }
    }
    // Should output the source file and line number and extract
    //   the expression from the assert call
    throw_error("assertion failed: got " +
                get_full_type(actual) + ":|" + actual + "|, expected " +
                get_full_type(expected) + ":|" + expected + "|" +
                (message ? " (" + message + ")" : ""));
}

function test_number()
{
    assert(parseInt("123"), 123);
    assert(parseInt("  123r"), 123);
    assert(parseInt("0x123"), 0x123);
    assert(parseInt("0o123"), 0);
    assert(+"  123   ", 123);
    assert(+"0b111", 7);
    assert(+"0o123", 83);
    assert(parseFloat("2147483647"), 2147483647);
    assert(parseFloat("2147483648"), 2147483648);
    assert(parseFloat("-2147483647"), -2147483647);
    assert(parseFloat("-2147483648"), -2147483648);
    assert(parseFloat("0x1234"), 0);
    assert(parseFloat("Infinity"), Infinity);
    assert(parseFloat("-Infinity"), -Infinity);
    assert(parseFloat("123.2"), 123.2);
    assert(parseFloat("123.2e3"), 123200);
    assert(Number.isNaN(Number("+")));
    assert(Number.isNaN(Number("-")));
    assert(Number.isNaN(Number("\x00a")));

    assert((1-2**-53).toString(12), "0.bbbbbbbbbbbbbba");
    assert((1000000000000000128).toString(), "1000000000000000100");
    assert((1000000000000000128).toFixed(0), "1000000000000000128");
    assert((25).toExponential(0), "3e+1");
    assert((-25).toExponential(0), "-3e+1");
    assert((2.5).toPrecision(1), "3");
    assert((-2.5).toPrecision(1), "-3");
    assert((25).toPrecision(1) === "3e+1");
    assert((1.125).toFixed(2), "1.13");
    assert((-1.125).toFixed(2), "-1.13");
    assert((0.5).toFixed(0), "1");
    assert((-0.5).toFixed(0), "-1");
    assert((-1e-10).toFixed(0), "-0");

    assert((1.3).toString(7), "1.2046204620462046205");
    assert((1.3).toString(35), "1.ahhhhhhhhhm");
}

test_number();
