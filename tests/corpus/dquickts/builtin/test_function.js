// Imported from dquickts tests/test_builtin.js (test_function) by
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

function assert_throws(expected_error, func)
{
    var err = false;
    try {
        func();
    } catch(e) {
        err = true;
        if (!(e instanceof expected_error)) {
            // Should output the source file and line number and extract
            //   the expression from the assert_throws() call
            throw_error("unexpected exception type");
            return;
        }
    }
    if (!err) {
        // Should output the source file and line number and extract
        //   the expression from the assert_throws() call
        throw_error("expected exception");
    }
}

/*----------------*/
function my_func(a, b)
{
    return a + b;
}

function test_function()
{
    function f(a, b) {
        var i, tab = [];
        tab.push(this);
        for(i = 0; i < arguments.length; i++)
            tab.push(arguments[i]);
        return tab;
    }
    function constructor1(a) {
        this.x = a;
    }

    var r, g;

    r = my_func.call(null, 1, 2);
    assert(r, 3, "call");

    r = my_func.apply(null, [1, 2]);
    assert(r, 3, "apply");

    r = (function () { return 1; }).apply(null, undefined);
    assert(r, 1);

    assert_throws(TypeError, (function() {
        Reflect.apply((function () { return 1; }), null, undefined);
    }));

    r = new Function("a", "b", "return a + b;");
    assert(r(2,3), 5, "function");

    g = f.bind(1, 2);
    assert(g.length, 1);
    assert(g.name, "bound f");
    assert(g(3), [1,2,3]);

    g = constructor1.bind(null, 1);
    r = new g();
    assert(r.x, 1);
}

test_function();
