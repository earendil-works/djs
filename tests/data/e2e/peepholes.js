// Code the 0086 peepholes and loop rotation rewrite: the results must not change.
function updates() {
  let i = 0x7fffffff;
  i++;
  ++i;
  let j = -0x80000000;
  j--;
  --j;
  let f = 1.5;
  f++;
  let s = "41";
  s++;
  let u;
  u++;
  let n = null;
  n--;
  let b = 2n ** 64n;
  b++;
  let c = -1n;
  c--;
  ++c;
  let t = true;
  t++;
  let k = 5;
  const vals = [k++, ++k, k--, --k, k];
  console.log(i, j, f, s, u, n, b, c, t, vals);
}
updates();

function objects() {
  const log = [];
  let o = { valueOf() { log.push("valueOf"); return 10; } };
  o++;
  console.log(o, typeof o, log.join());
  let p = { valueOf() { return "7"; } };
  p--;
  console.log(p);
  let q = { valueOf() { return 3n; } };
  q++;
  console.log(q);
  // A throwing conversion leaves the variable alone.
  const bad = { valueOf() { throw new Error("no"); } };
  let r = bad;
  try { r++; } catch (e) { console.log(e.message, r === bad); }
  // ToPrimitive gives a Symbol: ToNumeric throws, the variable keeps the object.
  const sym = { valueOf() { return Symbol("s"); } };
  let w = sym;
  try { w++; } catch (e) { console.log(e.name, w === sym); }
  let y = Symbol("y");
  try { y++; } catch (e) { console.log(e.name, typeof y); }
  const v = "x";
  try { v++; } catch (e) { console.log(e.name); }
}
objects();

function stores() {
  const o = { c: 0 };
  let x;
  x = 5;
  o.c = x;
  o.c += 2;
  o.c++;
  o["d"] = 1;
  o["d"] += 3;
  o["d"]--;
  const a = [1, 2, 3];
  let i = 0;
  a[i++] = 10;
  a[i] |= 4;
  a[i]++;
  let z = (o.e = 9);
  console.log(JSON.stringify(o), a, z, i);
  const setter = { set v(n) { console.log("set", n); }, get v() { return 1; } };
  setter.v = 3;
  setter.v += 1;
  setter.v++;
}
stores();

function loops() {
  let out = [];
  for (let i = 0; i < 4; i++) {
    if (i === 1) continue;
    if (i === 3) break;
    out.push(i);
  }
  let n = 0;
  while (n < 5) n += 2;
  out.push(n);
  let m = 3;
  while (true) {
    if (!m) break;
    m--;
  }
  out.push(m);
  for (;;) { out.push("once"); break; }
  for (let k = 0; ; k++) { if (k > 2) { out.push(k); break; } }
  let fns = [];
  for (let k = 0; k < 3; k++) fns.push(() => k);
  out.push(fns.map((f) => f()).join(""));
  outer: for (let a = 0; a < 3; a++) {
    for (let b = 0; b < 3; b++) {
      if (b === 1) continue outer;
      if (a === 2) break outer;
      out.push(`${a}${b}`);
    }
  }
  let w = 0;
  while (w < 3) { w++; if (w === 2) continue; out.push("w" + w); }
  do { w--; } while (w > 0);
  out.push(w);
  for (let e = 0; e < 2; e++) {
    try { if (e) throw new Error("e" + e); } catch (err) { out.push(err.message); }
  }
  console.log(out.join(" "));
}
loops();

function conditions(a, b, c) {
  let r = "";
  if (a || b) r += "1";
  if (a && b) r += "2";
  if ((a || b) && c) r += "3";
  if (a || (b && c)) r += "4";
  if (!a) r += "5";
  if (!(a || b)) r += "6";
  if (a === undefined || a === null) r += "7";
  if (b !== undefined) r += "8";
  while (a && !b) { r += "9"; break; }
  const v = a || b;
  return r + ":" + v + ":" + (c ? "c" : "-");
}
console.log(conditions(0, 1, 1), conditions(1, 1, 0), conditions(undefined, undefined, 1), conditions(null, 0, 0));

function returns(x) {
  const p = x * 2;
  if (p > 4) return p;
  return p > 2 ? "big" : "small";
}
console.log(returns(1), returns(2), returns(3));

// The line of an error in a rotated loop's test and update.
function where() {
  const lines = [];
  for (let i = 0; i < 2; i++) {
    try {
      for (let j = 0; j < (i ? null.x : 1); j++) {}
    } catch (e) { lines.push(e.constructor.name); }
  }
  return lines;
}
console.log(where());
