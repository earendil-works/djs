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

/**
 * The bytecode backend's inlining (0230): `Checked` and `Clamp` become expressions in
 * the caller; `Looping` (a loop) and `Big` (too long) stay calls.
 * @builtin Example.inlining
 */
function Inlining(x: Value, n: f64): f64 {
  const a: f64 = Clamp(Checked(x), n + 1);
  const b: f64 = Looping(a);
  return Big(a, b) + Clamp(b, n);
}

/** Throws for an object, `throw` as `%Throw(...)`. */
function Checked(v: Value): f64 {
  if (Leaf(v)) {
    throw NewError("object");
  }
  return 1;
}

/** A local and an `if` / `else`; `lo` is assigned, so it gets a temporary. */
function Clamp(v: f64, hi: f64): f64 {
  let lo: f64 = 0;
  lo = lo + 1;
  const p: f64 = v * 2;
  if (p < lo) {
    return lo;
  } else if (p > hi) {
    return hi;
  }
  return p;
}

function Looping(v: f64): f64 {
  let r: f64 = v;
  while (r > 10) {
    r = r - 1;
  }
  return r;
}

function Big(a: f64, b: f64): f64 {
  const s: f64 = a + b + a + b + a + b + a + b + a + b + a + b + a + b + a + b;
  const t: f64 = s * s * s * s * s * s * s * s * s * s * s * s * s * s * s * s * s * s;
  return s + t + a + b + a + b + a + b + a + b + a + b + a + b + a + b + a + b + a;
}
