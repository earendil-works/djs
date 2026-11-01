// Imported from dquickts tests/test_builtin.js (test_immutable_array_buffer) by
// scripts/difftest-import-dquickts.py; edit the script, not this file.
// difftest: skip-djs dynamic code (eval/Function) is a djs non-goal

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

function test_immutable_array_buffer()
{
    var src, buf, a, calls, r;
    var tracked = { valueOf() { calls.push("valueOf"); return 1; } };

    /* transferToImmutable() detaches the source */
    src = new ArrayBuffer(4, { maxByteLength: 8 });
    new Uint8Array(src).set([1, 2, 3, 4]);
    buf = src.transferToImmutable(6);
    assert(src.detached, true);
    assert(buf.immutable, true);
    assert(buf.resizable, false);
    assert(new Uint8Array(buf).join(), "1,2,3,4,0,0");
    assert(new Uint8Array(new Uint8Array([5, 6]).buffer.transferToImmutable(1)).join(), "5");
    assert(new ArrayBuffer(1).immutable, false);
    assert_throws(TypeError, () => Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, "immutable").get.call(new SharedArrayBuffer(1)));

    /* an immutable ArrayBuffer is read-only and cannot be detached nor resized */
    for (var f of [b => b.resize(0), b => b.transfer(), b => b.transferToFixedLength(),
                   b => b.transferToImmutable()]) {
        assert_throws(TypeError, () => f(buf));
        assert(buf.detached, false);
    }
    calls = [];
    assert_throws(TypeError, () => buf.resize(tracked));
    assert(calls.length, 0);

    /* slicing reads the buffer: the source can be immutable */
    assert(buf.slice(1, 3).immutable, false);
    assert(new Uint8Array(buf.slice(1, 3)).join(), "2,3");
    r = buf.sliceToImmutable(-3);
    assert(r.immutable, true);
    assert(new Uint8Array(r).join(), "4,0,0");
    /* sliceToImmutable() ignores the species constructor */
    src = new ArrayBuffer(4);
    src.constructor = { [Symbol.species]: function() { throw Error("species"); } };
    assert(src.sliceToImmutable(1).byteLength, 3);
    src.constructor = { [Symbol.species]: function() { return buf.sliceToImmutable(); } };
    assert_throws(TypeError, () => src.slice());
    /* shrinking the source below the end is a RangeError */
    src = new ArrayBuffer(8, { maxByteLength: 8 });
    assert_throws(RangeError, () => src.sliceToImmutable(0, { valueOf() { src.resize(2); return 4; } }));
    /* detaching it is a TypeError */
    assert_throws(TypeError, () => src.sliceToImmutable({ valueOf() { src.transfer(); return 0; } }));

    a = new Uint8Array(buf);
    /* the elements are non-writable, non-configurable data properties */
    assert(Reflect.set(a, "0", 9), false);
    assert(Reflect.set(a, "10", 9), false);
    calls = [];
    assert(Reflect.set(a, "1", tracked), false);
    assert(calls.length, 0);
    assert(Reflect.set(Object.create(a), "0", 9), false);
    assert(Function("a", "a[0] = 9; a[-1] = 9; return a[0]")(a), 1);
    assert_throws(TypeError, () => { a[10] = 1; });
    assert(Reflect.defineProperty(a, "0", { value: 1 }), true);
    assert(Reflect.defineProperty(a, "0", { value: "1" }), false);
    assert(Reflect.defineProperty(a, "0", { value: 2 }), false);
    assert(Reflect.defineProperty(a, "0", { writable: true }), false);
    assert(Reflect.defineProperty(a, "0", { configurable: true }), false);
    assert(Reflect.defineProperty(a, "0", { enumerable: false }), false);
    assert(Reflect.defineProperty(a, "0", { get() { return 1; } }), false);
    assert(Reflect.defineProperty(a, "0", { writable: false, configurable: false, enumerable: true }), true);
    assert(Object.isFrozen(Object.freeze(new Uint8Array(buf))), true);

    /* mutating methods throw before reading their arguments */
    for (f of [() => a.fill(tracked), () => a.set([tracked], tracked),
               () => a.copyWithin(tracked, 0), () => a.reverse(), () => a.sort(),
               () => a.setFromHex("ff"), () => a.setFromBase64("AA=="),
               () => new DataView(buf).setUint8(tracked, tracked),
               () => new DataView(buf).setBigInt64(tracked, 1n),
               () => Atomics.store(new Int8Array(buf), tracked, tracked),
               () => Atomics.add(new Int8Array(buf), tracked, tracked),
               () => Atomics.compareExchange(new Int8Array(buf), tracked, 1, 2)]) {
        calls = [];
        assert_throws(TypeError, f);
        assert(calls.length, 0);
    }
    assert(a.join(), "1,2,3,4,0,0");

    /* reading and copying are allowed */
    assert(a.toReversed().join(), "0,0,4,3,2,1");
    assert(a.toSorted().join(), "0,0,1,2,3,4");
    assert(a.with(0, 7).join(), "7,2,3,4,0,0");
    assert(a.slice(2, 4).join(), "3,4");
    assert(a.subarray(2).buffer, buf);
    assert(a.map(x => x * 2).join(), "2,4,6,8,0,0");
    assert(Atomics.load(new Int8Array(buf), 2), 3);
    assert(new DataView(buf).getUint16(0, true), 0x201);

    /* results written by TypedArray methods must be mutable */
    class Immutable extends Uint8Array {
        constructor(n) { super(typeof n === "number" ? new ArrayBuffer(n).transferToImmutable() : n); }
    }
    var im = new Immutable(4);
    assert_throws(TypeError, () => Immutable.from([1, 2]));
    assert_throws(TypeError, () => Immutable.of(1, 2));
    assert_throws(TypeError, () => im.map(x => x));
    assert_throws(TypeError, () => im.filter(x => true));
    assert_throws(TypeError, () => im.slice());
    assert(im.subarray(1) instanceof Immutable, true);
}

test_immutable_array_buffer();
