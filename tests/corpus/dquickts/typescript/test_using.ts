// Imported from dquickts tests/test_typescript.ts (test_using) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// 4 assertSyntaxError checks moved to test_using_syntax_N cases; 1 other evalTS lines dropped.

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

/* TypeScript 5.2 'using' and 'await using' declarations */
async function test_using() {
    const log: string[] = [];
    function res(name: string): Disposable {
        return { [Symbol.dispose]() { log.push(name); } };
    }
    function f(): number {
        using a: Disposable = res("a"), b: Disposable | null = null;
        using c = res("c") as Disposable;
        for (using x of [res("x")] as Disposable[])
            log.push("loop");
        return log.push("body");
    }
    assert(f(), 3);
    assert(log.join(), "loop,x,body,c,a");
    log.length = 0;
    await (async (): Promise<void> => {
        await using d: AsyncDisposable = {
            async [Symbol.asyncDispose](): Promise<void> { log.push("d"); },
        };
        using s = new DisposableStack();
        s.defer((): void => { log.push("s"); });
    })();
    assert(log.join(), "s,d");
    /* 'using' is still an identifier */
}

await test_using();
