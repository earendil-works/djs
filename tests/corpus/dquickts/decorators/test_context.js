// Imported from dquickts tests/test_decorators.js (test_context) by
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

function test_context() {
    const ctxs = {};
    const save = (value, ctx) => { ctxs[String(ctx.name)] = { value, ctx }; };
    @save class C {
        @save m() { return "m"; }
        @save static sm() {}
        @save get g() { return this.v; }
        @save set s(v) { this.v = v; }
        @save f = 1;
        @save accessor a = 2;
        @save #pm() { return "pm"; }
        @save get #pg() { return "pg"; }
        @save set #ps(v) { this.pv = v; }
        @save #pf = 3;
        @save accessor #pa = 4;
        static test(o) { return [o.#pm(), o.#pg, o.#pf, o.#pa]; }
    }
    const c = new C();
    let ctx = ctxs.C.ctx;
    assert(ctx.kind, "class");
    assert(ctxs.C.value, C);
    assert("static" in ctx || "private" in ctx || "access" in ctx, false);
    assert(typeof ctx.addInitializer, "function");
    ctx = ctxs.m.ctx;
    assert([ctx.kind, ctx.name, ctx.static, ctx.private].join(), "method,m,false,false");
    assert(ctxs.m.value, C.prototype.m);
    assert(ctx.access.get(c), C.prototype.m);
    assert(ctx.access.has(c) && !ctx.access.has({}), true);
    assert("set" in ctx.access, false);
    assertThrows(TypeError, () => ctx.access.get(1));
    assert(ctxs.sm.ctx.static, true);
    assert(ctxs.g.ctx.kind + ctxs.s.ctx.kind, "gettersetter");
    ctxs.s.ctx.access.set(c, 5);
    assert(ctxs.g.ctx.access.get(c), 5);
    assert("get" in ctxs.s.ctx.access, false);
    ctx = ctxs.f.ctx;
    assert(ctx.kind, "field");
    assert(ctxs.f.value, undefined);
    assert(ctx.access.get(c), 1);
    ctx.access.set(c, 10);
    assert(c.f, 10);
    ctx = ctxs.a.ctx;
    assert(ctx.kind, "accessor");
    assert(typeof ctxs.a.value.get + typeof ctxs.a.value.set, "functionfunction");
    assert(ctx.access.get(c), 2);
    /* private elements */
    ctx = ctxs["#pm"].ctx;
    assert([ctx.kind, ctx.name, ctx.private].join(), "method,#pm,true");
    assert(ctx.access.get(c)(), "pm");
    assert(ctx.access.has(c) && !ctx.access.has({}), true);
    assertThrows(TypeError, () => ctx.access.get({}));
    assert(ctxs["#pg"].value.name, "get #pg");
    assert(ctxs["#pg"].ctx.access.get(c), "pg");
    assert(ctxs["#ps"].value.name, "set #ps");
    ctxs["#ps"].ctx.access.set(c, 6);
    assert(c.pv, 6);
    assert(ctxs["#pf"].ctx.access.get(c), 3);
    ctxs["#pf"].ctx.access.set(c, 30);
    assert(ctxs["#pa"].ctx.access.get(c), 4);
    ctxs["#pa"].ctx.access.set(c, 40);
    assert(C.test(c).join(), "pm,pg,30,40");
    /* the metadata object is shared */
    assert(ctxs.m.ctx.metadata, ctxs.C.ctx.metadata);
    assert(C[Symbol.metadata], ctxs.C.ctx.metadata);
    /* the receiver of '@a.b' is 'a' */
    let recv;
    const obj = { dec() { recv = this; } };
    class D { @obj.dec m() {} }
    assert(recv, obj);
    /* computed keys and symbols */
    const sym = Symbol("sym");
    const names = [];
    const n = (v, ctx) => { names.push(ctx.name); };
    class E { @n [sym]() {} @n ["a" + 1]() {} @n [2] = 1; @n accessor [String(sym)] }
    assert(names[0], sym);
    assert(names.slice(1).join(), "a1,Symbol(sym),2");
}

test_context();
