// Constructs with runtime meaning: enums, parameter properties, namespaces.
enum Color { Red, Green = 5, Blue }
enum Dir { Up = "UP", Down = "DOWN" }
console.log(Color.Red, Color.Green, Color.Blue, Color[5], Dir.Up);
class Account {
  private static count = 0;
  constructor(public readonly owner: string, protected balance: number = 0) { Account.count++; }
  deposit(amount: number): this { this.balance += amount; return this; }
  get summary(): string { return `${this.owner}: ${this.balance}`; }
  static total(): number { return Account.count; }
}
const a = new Account("ada").deposit(10).deposit(5);
console.log(a.summary, Account.total(), a);
namespace Geometry {
  export const unit = 1;
  export function area(r: number): number { return Math.round(Math.PI * r * r * 100) / 100; }
}
console.log(Geometry.area(2), Geometry.unit);
abstract class Shape { abstract area(): number; describe(): string { return `area ${this.area()}`; } }
class Square extends Shape { constructor(private side: number) { super(); } area(): number { return this.side ** 2; } }
console.log(new Square(3).describe());
