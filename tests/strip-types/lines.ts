// Constructs whose erasure spans lines.
function multi<
  T,
>(x: T) {
  return x;
}
const arrow = (
  a: number,
): string => String(a);
const arrow2 = (a: number)
  : number => a;
class C {
  method<
    U,
  >(u: U) { return u; }
}
