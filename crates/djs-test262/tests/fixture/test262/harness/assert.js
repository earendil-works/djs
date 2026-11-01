// Minimal stand-in for test262's assert.js.
function assert(value, message) {
  if (value !== true) throw new Test262Error(message || "assertion failed");
}
assert.sameValue = function (actual, expected, message) {
  if (!Object.is(actual, expected)) {
    throw new Test262Error((message || "") + " Expected SameValue(" + String(actual) + ", " + String(expected) + ")");
  }
};
assert.throws = function (C, fn) {
  try {
    fn();
  } catch (e) {
    if (e.constructor !== C) throw new Test262Error("wrong error constructor");
    return;
  }
  throw new Test262Error("Expected a " + C.name + " to be thrown");
};
