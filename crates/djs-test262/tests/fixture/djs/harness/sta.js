// Minimal stand-in for test262's sta.js, using only what djs runs today.
function Test262Error(message) {
  this.message = message || "";
}
function $DONOTEVALUATE() {
  throw "Test262: This statement should not be evaluated.";
}
