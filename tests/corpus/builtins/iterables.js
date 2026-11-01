// Builtins over user iterables (0034): fromEntries, groupBy, Array.from, AggregateError.
function iter(values, log) {
  let i = 0;
  return {
    [Symbol.iterator]() {
      return {
        next() { return i < values.length ? { value: values[i++], done: false } : { value: undefined, done: true }; },
        return() { log.push("closed"); return {}; },
      };
    },
  };
}
const log = [];
console.log(Object.entries(Object.fromEntries(iter([["a", 1], ["b", 2], [3, "c"]], log))));
const g = Object.groupBy(iter([1, 2, 3, 4, 5, 6], log), (v) => (v % 2 ? "odd" : "even"));
console.log(Object.getPrototypeOf(g), Object.keys(g), g.odd, g.even);
console.log(Array.from(iter([1, 2, 3], log), (v) => v * 2), Array.from(iter(["x"], log)));
const e = new AggregateError(iter([new Error("a"), 2], log), "many", { cause: "c" });
console.log(e.message, e.errors.length, e.errors[1], e.cause, e instanceof AggregateError, e instanceof Error, String(e), AggregateError.length);
try { Object.fromEntries(iter([1], log)); } catch (x) { console.log(String(x), log.join()); }
try { Array.from(iter([1, 2], log), (v) => { throw new Error("map " + v); }); } catch (x) { console.log(String(x), log.join()); }
try { Object.groupBy(iter([1], log), () => { throw new RangeError("cb"); }); } catch (x) { console.log(String(x), log.join()); }
try { Object.fromEntries(5); } catch (x) { console.log(String(x)); }
try { new AggregateError(5); } catch (x) { console.log(String(x)); }
try { Object.fromEntries({ [Symbol.iterator]() { return { next() { return 1; } }; } }); } catch (x) { console.log(String(x)); }
