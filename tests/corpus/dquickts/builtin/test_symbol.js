// Imported from dquickts tests/test_builtin.js (test_symbol) by
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

function test_symbol()
{
    var a, b, obj, c;
    a = Symbol("abc");
    obj = {};
    obj[a] = 2;
    assert(obj[a], 2);
    assert(typeof obj["abc"], "undefined");
    assert(String(a), "Symbol(abc)");
    b = Symbol("abc");
    assert(a == a);
    assert(a === a);
    assert(a != b);
    assert(a !== b);

    b = Symbol.for("abc");
    c = Symbol.for("abc");
    assert(b === c);
    assert(b !== a);

    assert(Symbol.keyFor(b), "abc");
    assert(Symbol.keyFor(a), undefined);

    a = Symbol("aaa");
    assert(a.valueOf(), a);
    assert(a.toString(), "Symbol(aaa)");

    b = Object(a);
    assert(b.valueOf(), a);
    assert(b.toString(), "Symbol(aaa)");
}

test_symbol();
