function Point(x, y) { this.x = x; this.y = y; }
const p = new Point(1, 2);
console.log(p, p instanceof Point, p.constructor === Point);
function Weird() { return { replaced: true }; }
console.log(new Weird());
function Target() { return new.target === Target; }
console.log(new Target() instanceof Target, Target());
const arrow = () => 1;
new arrow();
