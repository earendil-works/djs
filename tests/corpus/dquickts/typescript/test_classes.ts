// Imported from dquickts tests/test_typescript.ts (test_classes) by
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

function test_classes() {
    abstract class Shape {
        abstract area(): number;
        abstract readonly kind: string;
        toString(): string { return this.kind + this.area(); }
    }
    class Square extends Shape implements Iterable<number> {
        readonly kind = "sq";
        [key: string]: unknown;
        declare extra: number;
        static readonly count: number = 0;
        private static secret?: string;
        protected optional?: number;
        definite!: number;
        constructor(private side: number) { super(); }
        area(): number { return this.side ** 2; }
        public override toString(): string { return "[" + super.toString() + "]"; }
        *[Symbol.iterator](): Iterator<number> { yield this.side; }
        method?(): void;
        overload(x: string): string;
        overload(x: number): number;
        overload(x: unknown): unknown { return x; }
    }
    const sq = new Square(3);
    assert(String(sq), "[sq9]");
    assert([...sq][0], 3);
    assert(sq.overload(1), 1);
    assert("extra" in sq, false, "'declare' field emits nothing");
    assert("optional" in sq && "definite" in sq, true, "typed fields are fields");
    assert(typeof sq.method, "undefined");
    assert(Object.keys(sq).join(), "side,kind,optional,definite");

    /* generic classes and expressions */
    const Box = class<T> { constructor(public v: T) {} };
    assert(new Box<number>(2).v, 2);
    class Pair<in out K, const V = unknown> extends Map<K, V> {}
    assert(new Pair<string, number>().size, 0);
}

test_classes();
