// Imported from dquickts tests/test_builtin.js (test_base64) by
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

function test_base64()
{
    assert(btoa(""), "");
    assert(btoa("f"), "Zg==");
    assert(btoa("fo"), "Zm8=");
    assert(btoa("foo"), "Zm9v");
    assert(btoa("\xff\xfe\x00"), "//4A");
    assert(btoa(123), "MTIz");
    assert(atob(""), "");
    assert(atob("Zm9v"), "foo");
    assert(atob("Zg=="), "f");
    assert(atob("Zg"), "f");
    assert(atob(" Zm 9v\n\t\f\r"), "foo");
    assert(atob("Zh=="), "f"); /* non zero trailing bits are ignored */
    assert(atob("//4A"), "\xff\xfe\x00");
    assert(atob(btoa("\x00\x80\xff")), "\x00\x80\xff");
    assert(atob(null), "\x9e\xe9e"); /* "null" */
    for (var s of ["Z", "Zg=", "Zm9v=", "Zg==Zg==", "Z===", "Zm-v", "Zm9\u0100", "Zm9v\u00e9"]) {
        try {
            atob(s);
            throw_error("atob: expected exception for " + JSON.stringify(s));
        } catch (e) {
            assert(e instanceof DOMException, true, s);
            assert(e.name, "InvalidCharacterError");
            assert(e.code, DOMException.INVALID_CHARACTER_ERR);
        }
    }
    try {
        btoa("\u0100");
        throw_error("btoa: expected exception");
    } catch (e) {
        assert(e instanceof DOMException);
        assert(e.code, 5);
    }
    assert_throws(TypeError, () => btoa());
    assert_throws(TypeError, () => atob());
}

test_base64();
