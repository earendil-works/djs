// Classes: modifiers, fields, signatures, heritage.
abstract class Base<T> implements Iterable<T> {
  declare d: number;
  abstract a: string;
  private readonly p = 1;
  protected static s?: number;
  public override toString(): string { return ""; }
  [k: string]: unknown;
  m?(): void;
  over(x: string): void;
  over(x: unknown): void {}
  abstract am(): void;
  get g(): number { return 1; }
  set g(v: number) {}
  private [Symbol.iterator](): Iterator<T> { return [][Symbol.iterator](); }
  definite!: number;
  static {
    type Inner = 1;
  }
}
class Derived extends Base<number> {
  declare x: string
  am() {}
}
const K = class implements Iterable<number> {
  [Symbol.iterator]() { return [][Symbol.iterator](); }
};
class G<in out T, const U extends readonly unknown[] = []> {}
