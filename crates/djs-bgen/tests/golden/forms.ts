// Every statement and expression form of the subset.

/**
 * @builtin Example.prototype.all
 */
function AllForms(this: Value, a: Value, b?: Value): Value {
  const self: Value = this;
  let n: f64 = 0.5;
  let i: i32 = -1;
  let u: u32 = 0xff;
  let s: Str = "quote \" backslash \\ newline \n tab \t é \u{1F600} \ud800";
  if (Leaf(a)) {
    n += 1;
  } else if (Leaf(b)) {
    n -= 1;
  } else {
    n *= 2;
  }
  if (Leaf(self)) n /= 2;
  while (n > 10) {
    n = n % 7;
  }
  do {
    i++;
  } while (i < 3);
  for (let k: u32 = 0; k < 4; k++) {
    if (k === 2) {
      continue;
    }
    u = u * 2 + k;
  }
  for (;;) {
    break;
  }
  i = (i & 7) | (i << 2) ^ ~i >> 1;
  i <<= 1;
  i >>= 1;
  i |= 1;
  i &= 15;
  i ^= 3;
  const z: u32 = i >>> 0;
  const neg: f64 = - -n;
  const c: boolean = !(n >= 1) && n <= 2 || n < 0;
  s += "x";
  try {
    User(a, 1, "two", s);
  } catch (e) {
    throw e;
  } finally {
    Helper(z, u);
  }
  try {
    Leaf(null);
  } catch {
    --i;
  }
  {
    const inner: boolean = c ? true : false;
    if (inner !== c) {
      return undefined;
    }
  }
  return neg === 0 ? Make() : s;
}

/** A leaf helper that the reentrant function calls. */
function Helper(a: u32, b: u32): boolean {
  return a === b;
}

/**
 * Registered as %Callback.
 * @intrinsic
 */
function Callback(f: Value): Value {
  return User(f);
}
