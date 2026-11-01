// The Rust backend (0063): the forms of the subset in @native builtins.

/**
 * Loops, typed locals with checked integer arithmetic, resume points in conditions and
 * arguments, a virtual rest parameter, an inlined helper and a leaf helper.
 * @builtin Example.prototype.loops
 * @native
 */
function NativeLoops(this: Value, f: Value, ...rest: Value[]): f64 {
  let i: i32 = 0;
  let u: u32 = 1;
  let sum: f64 = 0;
  const n: f64 = LengthOfArrayLike(rest);
  for (let k: f64 = 0; k < n; k++) {
    const v: Value = Get(rest, k);
    if (Leaf(v) && Leaf(User(f, v))) {
      continue;
    }
    i = i * 2 + 1;
    u = u - 1 + 2;
    sum += Twice(Inlined(v)) + (i < 3 ? 1 : 0);
    while (Leaf(User(f, k)) || i > 100) {
      i--;
      if (i < 0) {
        break;
      }
    }
  }
  do {
    i = i >> 1;
  } while (i > 0);
  return sum + (Leaf(this) ? Count(User(f)) : -1);
}

/**
 * try/catch around resume points, a materialized rest parameter, string literals
 * (one with a lone surrogate) and throw.
 * @intrinsic
 * @native
 */
function NativeTry(f: Value, ...rest: Value[]): Value {
  let r: Value = rest;
  try {
    r = User(f, "a\u00e9\ud800");
    if (r === null) {
      throw NewError("null");
    }
  } catch (e) {
    try {
      r = User(e);
    } catch {
      r = undefined;
    }
  }
  return r;
}

/** Inlined into its callers: small, internal, reentrant. */
function Inlined(v: Value): f64 {
  if (Leaf(v)) {
    return 1;
  }
  return LengthOfArrayLike(User(v));
}

/** A leaf helper: plain Rust. */
function Twice(x: f64): f64 {
  return x * 2;
}

/**
 * Not inlined (an intrinsic with too many statements would be; this one is called
 * through call_installed).
 * @intrinsic
 */
function Count(v: Value): f64 {
  let n: f64 = 0;
  let k: f64 = 0;
  while (k < 10) {
    if (Leaf(User(v, k))) {
      n++;
    }
    k++;
    k++;
    k++;
    k++;
  }
  return n;
}

/**
 * try/finally (0080): the finally block runs after the body, on an exception (then
 * rethrows), and before a return or a break that leaves it.
 * @builtin Fin
 * @native
 */
function Fin(v: Value): f64 {
  let n: f64 = 0;
  while (n < 5) {
    try {
      if (Leaf(User(v, n))) {
        break;
      }
      if (n > 3) {
        return n;
      }
    } catch (e) {
      User(e);
    } finally {
      n++;
    }
  }
  return n;
}

/**
 * A constructor (0080): new.target is the frame's.
 * @constructor Ctor
 * @native
 */
function Ctor(v: Value): Value {
  if (new.target === undefined) {
    return User(v);
  }
  return Make();
}

/**
 * The fused dense HasProperty + Get (D2090): fused when the true branch starts with the
 * Get of the same operands, not when it does something else first.
 * @intrinsic
 * @native
 */
function NativeHasGet(o: Value, f: Value): f64 {
  const n: f64 = LengthOfArrayLike(o);
  let sum: f64 = 0;
  for (let k: f64 = 0; k < n; k++) {
    if (HasProperty(o, k)) {
      User(f, Get(o, k), k);
    }
    if (HasProperty(o, k)) {
      User(f, k);
      sum += Leaf(Get(o, k)) ? 1 : 0;
    }
  }
  return sum;
}
