// Imported from dquickts tests/test_builtin.js (test_math) by
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

function test_math()
{
    var a;
    a = 1.4;
    assert(Math.floor(a), 1);
    assert(Math.ceil(a), 2);
    assert(Math.imul(0x12345678, 123), -1088058456);
    assert(Math.imul(0xB505, 0xB504), 2147441940);
    assert(Math.imul(0xB505, 0xB505), -2147479015);
    assert(Math.imul((-2)**31, (-2)**31), 0);
    assert(Math.imul(2**31-1, 2**31-1), 1);
    assert(Math.fround(0.1), 0.10000000149011612);
    assert(Math.hypot(), 0);
    assert(Math.hypot(-2), 2);
    assert(Math.hypot(3, 4), 5);
    assert(Math.abs(Math.hypot(3, 4, 5) - 7.0710678118654755) <= 1e-15);
    assert(Math.sumPrecise([1,Number.EPSILON/2,Number.MIN_VALUE]), 1.0000000000000002);
}

test_math();
