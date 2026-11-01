// Imported from dquickts tests/test_builtin.js (test_regexp) by
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

function test_regexp()
{
    var a, str;
    str = "abbbbbc";
    a = /(b+)c/.exec(str);
    assert(a[0], "bbbbbc");
    assert(a[1], "bbbbb");
    assert(a.index, 1);
    assert(a.input, str);
    a = /(b+)c/.test(str);
    assert(a, true);
    assert(/\x61/.exec("a")[0], "a");
    assert(/\u0061/.exec("a")[0], "a");
    assert(/\ca/.exec("\x01")[0], "\x01");
    assert(/\\a/.exec("\\a")[0], "\\a");
    assert(/\c0/.exec("\\c0")[0], "\\c0");

    a = /(\.(?=com|org)|\/)/.exec("ah.com");
    assert(a.index === 2 && a[0] === ".");

    a = /(\.(?!com|org)|\/)/.exec("ah.com");
    assert(a, null);

    a = /(?=(a+))/.exec("baaabac");
    assert(a.index === 1 && a[0] === "" && a[1] === "aaa");

    a = /(z)((a+)?(b+)?(c))*/.exec("zaacbbbcac");
    assert(a, ["zaacbbbcac","z","ac","a",,"c"]);

    a = eval("/\0a/");
    assert(a.toString(), "/\0a/");
    assert(a.exec("\0a")[0], "\0a");

    assert(/{1a}/.toString(), "/{1a}/");
    a = /a{1+/.exec("a{11");
    assert(a, ["a{11"]);

    /* test zero length matches */
    a = /(?:(?=(abc)))a/.exec("abc");
    assert(a, ["a", "abc"]);
    a = /(?:(?=(abc)))?a/.exec("abc");
    assert(a, ["a", undefined]);
    a = /(?:(?=(abc))){0,2}a/.exec("abc");
    assert(a, ["a", undefined]);
    a = /(?:|[\w])+([0-9])/.exec("123a23");
    assert(a, ["123a23", "3"]);
    a = /()*?a/.exec(",");
    assert(a, null);

    /* test \b escape */
    assert(/[\q{a\b}]/.test("a\b"), true);
    assert(/[\b]/.test("\b"), true);
    
    /* test case insensitive matching (test262 hardly tests it) */
    assert("aAbBcC#4".replace(/\p{Lower}/gu,"X"), "XAXBXC#4");

    assert("aAbBcC#4".replace(/\p{Lower}/gui,"X"), "XXXXXX#4");
    assert("aAbBcC#4".replace(/\p{Upper}/gui,"X"), "XXXXXX#4");
    assert("aAbBcC#4".replace(/\P{Lower}/gui,"X"), "XXXXXXXX");
    assert("aAbBcC#4".replace(/\P{Upper}/gui,"X"), "XXXXXXXX");
    assert("aAbBcC".replace(/[^b]/gui, "X"), "XXbBXX");
    assert("aAbBcC".replace(/[^A-B]/gui, "X"), "aAbBXX");

    assert("aAbBcC#4".replace(/\p{Lower}/gvi,"X"), "XXXXXX#4");
    assert("aAbBcC#4".replace(/\P{Lower}/gvi,"X"), "aAbBcCXX");
    assert("aAbBcC#4".replace(/[^\P{Lower}]/gvi,"X"), "XXXXXX#4");
    assert("aAbBcC#4".replace(/\P{Upper}/gvi,"X"), "aAbBcCXX");
    assert("aAbBcC".replace(/[^b]/gvi, "X"), "XXbBXX");
    assert("aAbBcC".replace(/[^A-B]/gvi, "X"), "aAbBXX");
    assert("aAbBcC".replace(/[[a-c]&&B]/gvi, "X"), "aAXXcC");
    assert("aAbBcC".replace(/[[a-c]--B]/gvi, "X"), "XXbBXX");
    
    assert("abcAbC".replace(/[\q{AbC}]/gvi,"X"), "XX");
    /* Note: SpiderMonkey and v8 may not be correct */
    assert("abcAbC".replace(/[\q{BC|A}]/gvi,"X"), "XXXX");
    assert("abcAbC".replace(/[\q{BC|A}--a]/gvi,"X"), "aXAX");

    /* case where lastIndex points to the second element of a
       surrogate pair */
    a = /(?:)/gu;
    a.lastIndex = 1;
    a.exec("🐱");
    assert(a.lastIndex, 0);

    a.lastIndex = 1;
    a.exec("a\udc00");
    assert(a.lastIndex, 1);

    a = /\u{10000}/vgd;
    a.lastIndex = 1;
    a = a.exec("\u{10000}_\u{10000}");
    assert(a.indices[0][0], 0);
    assert(a.indices[0][1], 2);

    /* syntax check */
    assert(/-&&/v.test("-&&"), true);
}

test_regexp();
