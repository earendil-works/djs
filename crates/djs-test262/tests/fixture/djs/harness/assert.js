// Minimal stand-in for test262's assert.js, using only what djs runs today.
function assert(value, message) {
  if (value !== true) throw new Test262Error(message || "assertion failed");
}
assert.sameValue = function (actual, expected, message) {
  if (actual !== expected) throw new Test262Error(message || "not the same value");
};
