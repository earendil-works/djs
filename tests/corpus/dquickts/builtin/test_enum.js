// Imported from dquickts tests/test_builtin.js (test_enum) by
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

function test_enum()
{
    var a, tab;
    a = {x:1,
         "18014398509481984": 1,
         "9007199254740992": 1,
         "9007199254740991": 1,
         "4294967296": 1,
         "4294967295": 1,
         y:1,
         "4294967294": 1,
         "1": 2};
    tab = Object.keys(a);
//    console.log("tab=" + tab.toString());
    assert(tab, ["1","4294967294","x","18014398509481984","9007199254740992","9007199254740991","4294967296","4294967295","y"], "keys");
}

test_enum();
