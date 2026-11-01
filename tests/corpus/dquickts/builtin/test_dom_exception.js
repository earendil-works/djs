// Imported from dquickts tests/test_builtin.js (test_dom_exception) by
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

function test_dom_exception()
{
    var e, d;
    e = new DOMException();
    assert(e.name, "Error");
    assert(e.message, "");
    assert(e.code, 0);
    assert(e instanceof Error);
    assert(Error.isError(e), true);
    assert(Error.isError(DOMException.prototype), false);
    assert(Object.getPrototypeOf(DOMException.prototype), Error.prototype);
    assert(Object.prototype.toString.call(e), "[object DOMException]");
    assert(String(e), "Error");
    e = new DOMException("msg", "NotFoundError");
    assert(e.name, "NotFoundError");
    assert(e.message, "msg");
    assert(e.code, 8);
    assert(String(e), "NotFoundError: msg");
    assert(typeof e.stack, "string");
    assert(Object.keys(e).length, 0);
    assert(new DOMException("m", "Custom").code, 0);
    assert(new DOMException(undefined, undefined).name, "Error");
    assert(new DOMException(1, 2).name, "2");
    assert(DOMException.length, 0);
    assert(DOMException.name, "DOMException");
    assert_throws(TypeError, () => DOMException());
    assert(DOMException.DATA_CLONE_ERR, 25);
    assert(DOMException.prototype.ABORT_ERR, 20);
    assert(e.TIMEOUT_ERR, 23);
    d = Object.getOwnPropertyDescriptor(DOMException, "INDEX_SIZE_ERR");
    assert(d.value === 1 && d.enumerable && !d.writable && !d.configurable);
    d = Object.getOwnPropertyDescriptor(DOMException.prototype, "name");
    assert(typeof d.get === "function" && d.enumerable && d.configurable && d.set === undefined);
    d = Object.getOwnPropertyDescriptor(globalThis, "DOMException");
    assert(d.writable && !d.enumerable && d.configurable);
    assert_throws(TypeError, () => DOMException.prototype.name);
    assert_throws(TypeError, () => Object.getOwnPropertyDescriptor(DOMException.prototype, "code").get.call({}));
    class MyError extends DOMException {
        constructor() { super("sub", "AbortError"); }
    }
    e = new MyError();
    assert(e instanceof MyError && e instanceof DOMException);
    assert(e.name, "AbortError");
    assert(e.code, 20);
}

test_dom_exception();
