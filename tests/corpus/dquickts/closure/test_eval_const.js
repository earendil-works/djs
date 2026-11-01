// difftest: skip-djs direct eval: access to the caller's scope is a non-goal (D014)
// Imported from dquickts tests/test_closure.js (test_eval_const) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

function assert(actual, expected, message) {
    if (arguments.length == 1)
        expected = true;

    if (actual === expected)
        return;

    if (actual !== null && expected !== null
    &&  typeof actual == 'object' && typeof expected == 'object'
    &&  actual.toString() === expected.toString())
        return;

    throw Error("assertion failed: got |" + actual + "|" +
                ", expected |" + expected + "|" +
                (message ? " (" + message + ")" : ""));
}

/*----------------*/
var log_str = "";

function log(str)
{
    log_str += str + ",";
}

function f(a, b, c)
{
    var x = 10;
    log("a="+a);
    function g(d) {
        function h() {
            log("d=" + d);
            log("x=" + x);
        }
        log("b=" + b);
        log("c=" + c);
        h();
    }
    g(4);
    return g;
}

function test_eval_const()
{
    const a = 1;
    var success = false;
    var f = function () {
        eval("a = 1");
    };
    try {
        f();
    } catch(e) {
        success = (e instanceof TypeError);
    }
    assert(success);
}

test_eval_const();
