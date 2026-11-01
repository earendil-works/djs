// Imported from dquickts tests/test_loop.js (test_for_in_proxy) by
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

function test_for_in_proxy() {
    let removed_key = "";
    let target = {}
    let proxy = new Proxy(target, {
        ownKeys: function() {
            return ["a", "b", "c"];
        },
        getOwnPropertyDescriptor: function(target, key) {
            if (removed_key != "" && key == removed_key)
                return undefined;
            else
                return { enumerable: true, configurable: true, value: this[key] };
        }
    });
    let str = "";
    for(let o in proxy) {
        str += " " + o;
        if (o == "a")
            removed_key = "b";
    }
    assert(str == " a c");
}

test_for_in_proxy();
