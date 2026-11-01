// JSON.stringify with indentation and a class instance with a getter (0090).
// bench: expect 3192820
class Point { constructor(x, y) { this.x = x; this.y = y; } get len() { return Math.hypot(this.x, this.y); } }
const rows = [];
for (let i = 0; i < 2000; i++) {
  rows.push({ id: i, name: "item " + i, size: i * 37, tags: ["a", "b"], at: new Point(i, i + 1) });
}
Object.defineProperty(rows[5], "computed", { get() { return 42; }, enumerable: true });
let total = 0;
for (let r = 0; r < 10; r++) total += JSON.stringify(rows, null, 2).length;
return total;
