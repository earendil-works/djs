// Imported from dquickts tests/test_typescript.ts (test_rejected) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// 12 assertSyntaxError checks moved to test_rejected_syntax_N cases; 1 other evalTS lines dropped.
// difftest: skip-djs dynamic code (eval/Function) is a djs non-goal

function assert(actual, expected, message) {
    if (arguments.length === 1)
        expected = true;

    if (typeof actual === typeof expected) {
        if (Object.is(actual, expected))
            return;
        if (typeof actual === 'object') {
            if (actual !== null && expected !== null
            &&  actual.constructor === expected.constructor
            &&  actual.toString() === expected.toString())
                return;
        }
    }
    throw Error("assertion failed: got |" + actual + "|" +
                ", expected |" + expected + "|" +
                (message ? " (" + message + ")" : ""));
}

function assertThrows(err, func)
{
    var ex;
    ex = false;
    try {
        func();
    } catch(e) {
        ex = true;
        assert(e instanceof err);
    }
    assert(ex, true, "exception expected");
}

function test_rejected() {
    /* TypeScript syntax is not JavaScript */
    /* unsupported constructs */
    /* parameter decorators need experimentalDecorators */
    /* TypeScript errors */
    /* direct eval() and Function() are JavaScript */
    assertThrows(SyntaxError, () => Function("let x: number = 1"));
}

test_rejected();
