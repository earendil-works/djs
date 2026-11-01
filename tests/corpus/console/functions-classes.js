// Functions and classes have no JSON form: String() prints their source.
function named(a, b) { return a + b; }
console.log(named);
console.log((x) => x * 2);
console.log(async function af() {});
class Point {
  constructor(x, y) { this.x = x; this.y = y; }
  get len() { return Math.hypot(this.x, this.y); }
}
console.log(Point);
// Instances serialize their own enumerable fields.
console.log(new Point(3, 4));
class WithField { a = 1; #secret = 2; static s = 3; }
console.log(new WithField());
console.log(Math.max, [].push);
