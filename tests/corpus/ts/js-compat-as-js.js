// Code valid in both JavaScript and TypeScript means the same in both: js-compat-as-js.js
// and js-compat-as-ts.ts are identical and must give the same result. From dquickts
// tests/test_typescript.ts (test_js_compatibility).
return (function () {
        var log = [];
        var type = 1, declare = 2, abstract = 3;
        var a = 1, b = 2, c = 3, x = 5, y = 6, z = 7;
        log.push(a < b, a < b > c, x ? (y) : z, typeof type);
        var t = true ? (y) : (z) => z;
        log.push(typeof t);
        var f = (p, q = 2) => p + q;
        log.push(f(1));
        class K { static s = 1; get g() { return 2; } readonly = 3; declare() { return 4; } }
        log.push(K.s + new K().g + new K().readonly + new K().declare());
        label: { break label; }
        var re = /[a-z]+/g;
        log.push("abc".match(re)[0], `t${a}`);
        var o = { async: 1, get: 2, set: 3, static: 4, type: 5 };
        log.push(o.async + o.get + o.set + o.static + o.type);
        function* g() { yield 1; }
        log.push([...g()].length);
        var n = a
        !b;
        log.push(n);
        return log.join();
    })();
