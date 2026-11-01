// Imported from dquickts tests/test_typescript.ts (test_asi_contextual_keywords) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.

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

function test_asi_contextual_keywords() {
    /* contextual keywords are identifiers where JavaScript allows them */
    var type = 1, declare = 2, abstract = 3, namespace = 4, module = 5;
    var readonly = 6, is = 7, asserts = 8, satisfies = 9, keyof = 10, infer = 11, global = 12;
    assert(type + declare + abstract + namespace + module + readonly + is +
           asserts + satisfies + keyof + infer + global, 78);
    /* a line break ends the statement as with tsc */
    let r: number[] = [];
    type
    Foo = 1;
    assert(Foo, 1);
    declare
    const fromDeclare = 2;
    assert(fromDeclare, 2);
    namespace
    Bar = 3;
    assert(Bar, 3);
    abstract
    class AfterAbstract {}
    assert(typeof AfterAbstract, "function");
    /* no line break before 'as' (tsc), but the type may follow one */
    let k = type as
        number;
    assert(k, 1);
    /* labels */
    type: for (;;) { break type; }
    declare: { r.push(1); }
    assert(r.length, 1);
    /* members named like modifiers */
    class M {
        readonly = 1;
        declare() { return 2; }
        abstract?: number;
        static public = 3;
        get<T>(x: T) { return x; }
        set?: number;
        is = 4;
    }
    const m = new M();
    assert(m.readonly + m.declare() + M.public + m.get(4) + m.is, 14);
    var Foo, Bar;
}

test_asi_contextual_keywords();
