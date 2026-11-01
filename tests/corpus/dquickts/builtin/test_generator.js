// Imported from dquickts tests/test_builtin.js (test_generator) by
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

function test_generator()
{
    function *f() {
        var ret;
        yield 1;
        ret = yield 2;
        assert(ret, "next_arg");
        return 3;
    }
    function *f2() {
        yield 1;
        yield 2;
        return "ret_val";
    }
    function *f1() {
        var ret = yield *f2();
        assert(ret, "ret_val");
        return 3;
    }
    function *f3() {
        var ret;
        /* test stack consistency with nip_n to handle yield return +
         * finally clause */
        try {
            ret = 2 + (yield 1);
        } catch(e) {
        } finally {
            ret++;
        }
        return ret;
    }
    var g, v;
    g = f();
    v = g.next();
    assert(v.value === 1 && v.done === false);
    v = g.next();
    assert(v.value === 2 && v.done === false);
    v = g.next("next_arg");
    assert(v.value === 3 && v.done === true);
    v = g.next();
    assert(v.value === undefined && v.done === true);

    g = f1();
    v = g.next();
    assert(v.value === 1 && v.done === false);
    v = g.next();
    assert(v.value === 2 && v.done === false);
    v = g.next();
    assert(v.value === 3 && v.done === true);
    v = g.next();
    assert(v.value === undefined && v.done === true);

    g = f3();
    v = g.next();
    assert(v.value === 1 && v.done === false);
    v = g.next(3);
    assert(v.value === 6 && v.done === true);
}

test_generator();
