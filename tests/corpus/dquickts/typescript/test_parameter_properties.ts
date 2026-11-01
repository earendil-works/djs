// Imported from dquickts tests/test_typescript.ts (test_parameter_properties) by
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

/* parameter properties: like tsc with useDefineForClassFields, they are
   defined before the other fields and assigned at the start of the
   constructor (after super() in a derived class) */
function test_parameter_properties() {
    const log: string[] = [];
    class Base {
        constructor(public b: number) {
            log.push("base:" + Object.keys(this).join());
        }
    }
    class Derived extends Base {
        field = (log.push("field:" + Object.keys(this).join() + ":" + this.x), 1);
        constructor(public x: number, protected readonly y: string = "dy",
                    private z?: number) {
            log.push("ctor");
            super(x * 2);
            log.push("after:" + Object.keys(this).join() + ":" + this.x);
        }
    }
    const d = new Derived(5);
    assert(log.join("|"), "ctor|base:b|field:b,x,y,z:undefined|after:b,x,y,z,field:5");
    assert(JSON.stringify(d), '{"b":10,"x":5,"y":"dy","field":1}');

    /* base class: fields see the parameter property fields, not values */
    class P {
        a = this.x;
        constructor(public x: number = 1, readonly r = this.a) {}
    }
    const p = new P();
    assert(p.a, undefined);
    assert(p.x, 1);
    assert(Object.keys(p).join(), "x,r,a");

    /* super() in an arrow function */
    class Q extends Base {
        constructor(public q: number) {
            const init = () => super(q);
            init();
        }
    }
    assert(new Q(4).q + new Q(4).b, 8);
}

test_parameter_properties();
