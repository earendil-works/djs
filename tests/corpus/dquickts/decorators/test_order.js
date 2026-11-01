// Imported from dquickts tests/test_decorators.js (test_order) by
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

function test_order() {
    const log = [];
    function dec(name) {
        log.push("eval " + name);
        return function (value, ctx) {
            log.push(name);
            ctx.addInitializer(function () { log.push("init " + name); });
            if (ctx.kind === "field")
                return (v) => { log.push("field init " + name); return v; };
            if (ctx.kind === "accessor")
                return { init(v) { log.push("accessor init " + name); return v; } };
        };
    }
    log.push("start");
    @dec("c1") @dec("c2")
    class C {
        @dec("f") f = log.push("f value");
        @dec("sf") static sf = log.push("sf value");
        @dec("m") m() {}
        @dec("sm") static sm() {}
        @dec("a") accessor a;
        @dec("sa") static accessor sa;
        static { log.push("static block"); }
        constructor() { log.push("ctor"); }
    }
    log.push("defined");
    new C();
    assert(log.join(),
           "start,eval c1,eval c2,eval f,eval sf,eval m,eval sm,eval a,eval sa," +
           /* static non-fields, instance non-fields, static fields,
              instance fields, class */
           "sm,sa,m,a,sf,f,c2,c1," +
           /* static method initializers, static elements, class initializers */
           "init sm,sf value,field init sf,init sf,accessor init sa,init sa," +
           "static block,init c2,init c1,defined," +
           /* instance method initializers, fields */
           "init m,f value,field init f,init f,accessor init a,init a,ctor");

    /* the decorators are applied from the last one, the initializers of
       the first decorator run first */
    const order = [];
    const d = (n) => () => (v) => { order.push(n); return v + n; };
    class D { @d("a") @d("b") x = ""; }
    assert(new D().x, "ab");
    assert(order.join(), "a,b");
}

test_order();
