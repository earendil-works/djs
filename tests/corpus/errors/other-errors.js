// Messages of RangeErrors, SyntaxErrors from JSON.parse, and friends.
// difftest: djs-expect recursion -> InternalError: stack overflow => recursion -> RangeError: Maximum call stack size exceeded
// difftest: no-stress recurses to a stack overflow: over 120 s in each stress mode (0053)
function attempt(label, fn) {
  try {
    const v = fn();
    console.log(label, "-> no error:", v);
  } catch (e) {
    console.log(label, "->", e);
  }
}
attempt("invalid array length", () => new Array(-1));
attempt("set invalid length", () => { [].length = 2 ** 32; });
attempt("toFixed digits", () => (1).toFixed(101));
attempt("toPrecision digits", () => (1).toPrecision(0));
attempt("toString radix", () => (1).toString(1));
attempt("repeat negative", () => "a".repeat(-1));
attempt("repeat infinity", () => "a".repeat(Infinity));
attempt("normalize form", () => "a".normalize("bad"));
attempt("fromCodePoint", () => String.fromCodePoint(0x110000));
attempt("invalid date toISOString", () => new Date(NaN).toISOString());
attempt("recursion", () => { const f = () => f(); f(); });
attempt("JSON.parse empty", () => JSON.parse(""));
attempt("JSON.parse trailing comma", () => JSON.parse("[1,]"));
attempt("JSON.parse single quotes", () => JSON.parse("{'a':1}"));
attempt("JSON.parse unterminated", () => JSON.parse('{"a":'));
attempt("JSON.parse garbage", () => JSON.parse("nope"));
attempt("JSON.parse ok", () => JSON.parse(' {"a":[1,2,{"b":null}]} ').a[2].b);
attempt("decodeURIComponent", () => decodeURIComponent("%"));
attempt("encodeURIComponent lone surrogate", () => encodeURIComponent("\ud800"));
attempt("BigInt from float", () => BigInt(1.5));
attempt("BigInt division by zero", () => 1n / 0n);
attempt("new Symbol", () => new Symbol());
