// TextEncoder and TextDecoder (0244). qjs has neither, so node is the oracle here:
// cases.expected is node's output (gen-expected.sh), and vm::encoding_tests requires djs
// to print the same. Only behavior where djs means to agree with node is here (error
// names, not messages); the deliberate differences are asserted in vm/encoding_tests.rs.
// The completion value is the transcript.
const out = [];
const hex = (s) => [...s].map((c) => c.codePointAt(0).toString(16)).join(" ");
const t = (name, f) => {
  try {
    const r = f();
    out.push(name + " " + JSON.stringify(r));
  } catch (e) {
    out.push(name + " throws " + e.name);
  }
};
const bytes = (...b) => new Uint8Array(b);
const desc = (o, k) => {
  const d = Object.getOwnPropertyDescriptor(o, k);
  return Object.entries(d).map(([a, b]) => a + "=" + (typeof b === "function" ? b.name + "/" + b.length : String(b))).join(",");
};

// The classes' shape.
for (const C of [TextEncoder, TextDecoder]) {
  t(C.name + " shape", () => [C.length, desc(C, "prototype"), Reflect.ownKeys(C).map(String)]);
  // (node has a util.inspect.custom method too)
  const keys = Reflect.ownKeys(C.prototype).filter((k) => typeof k === "string" || k === Symbol.toStringTag);
  t(C.name + " keys", () => keys.map(String));
  for (const k of keys) t(C.name + " " + String(k), () => desc(C.prototype, k));
  t(C.name + " tags", () => [String(new C()), Object.prototype.toString.call(C.prototype)]);
  t(C.name + " call", () => C());
}
t("encoder getter on prototype", () => TextEncoder.prototype.encoding);
t("decoder getter on prototype", () => TextDecoder.prototype.fatal);
t("encode on other", () => TextEncoder.prototype.encode.call({}, "x"));
t("decode on other", () => TextDecoder.prototype.decode.call(new TextEncoder()));
t("subclass", () => {
  class D extends TextDecoder {}
  const d = new D();
  return [d instanceof D, d.decode(bytes(65)), Object.getPrototypeOf(d) === D.prototype];
});

// Labels and options.
for (const l of ["utf-8", " UTF8\t", "\fUnicode-1-1-utf-8\n", "unicode11utf8", "unicode20utf8", "x-unicode20utf8", "utf-8\v", "", undefined, null, "utf\u0130-8", "utf-8\0"])
  t("label " + JSON.stringify(l), () => new TextDecoder(l).encoding);
t("label symbol", () => new TextDecoder(Symbol()));
t("options null", () => new TextDecoder("utf-8", null).fatal);
t("options number", () => new TextDecoder("utf-8", 1).fatal);
t("options string", () => new TextDecoder("utf-8", "x").fatal);
t("options function", () => {
  const f = () => {};
  f.fatal = 1;
  return new TextDecoder("utf-8", f).fatal;
});
t("getters", () => {
  const d = new TextDecoder("UTF8", { fatal: 1, ignoreBOM: "" });
  return [d.encoding, d.fatal, d.ignoreBOM, new TextDecoder().fatal, new TextEncoder().encoding];
});

// decode: inputs.
const d = new TextDecoder();
t("decode none", () => d.decode());
t("decode undefined", () => d.decode(undefined));
t("decode null", () => d.decode(null));
t("decode array", () => d.decode([65]));
t("decode string", () => d.decode("A"));
t("decode ArrayBuffer", () => d.decode(bytes(0x41, 0xe2, 0x82, 0xac).buffer));
t("decode DataView", () => d.decode(new DataView(bytes(0, 0x41, 0x42, 0).buffer, 1, 2)));
t("decode Uint16Array", () => d.decode(new Uint16Array([0x4241])));
t("decode subarray", () => d.decode(bytes(0x41, 0x42, 0x43).subarray(1)));
t("decode options null", () => d.decode(bytes(65), null));
t("decode options number", () => d.decode(bytes(65), 1));
t("decode detached buffer", () => {
  const b = new ArrayBuffer(2);
  b.transfer();
  return d.decode(b);
});
t("decode detached view", () => {
  const b = new ArrayBuffer(2);
  const v = new Uint8Array(b);
  b.transfer();
  return [d.decode(v), d.decode(new DataView(new ArrayBuffer(1)))];
});
t("decode resizable", () => {
  const b = new ArrayBuffer(4, { maxByteLength: 8 });
  new Uint8Array(b).set([0x61, 0x62, 0x63, 0x64]);
  const all = new Uint8Array(b), fixed = new Uint8Array(b, 1, 2), dv = new DataView(b, 1);
  const dvFixed = new DataView(b, 1, 2);
  const r = [d.decode(all), d.decode(fixed), d.decode(dv)];
  b.resize(6);
  r.push(d.decode(all), d.decode(dv));
  b.resize(2);
  r.push(d.decode(all), d.decode(dv), d.decode(fixed), d.decode(dvFixed));
  return r.map(hex);
});

// decode: BOM, errors, streaming.
t("bom", () => d.decode(bytes(0xef, 0xbb, 0xbf, 0x41)));
t("bom only", () => d.decode(bytes(0xef, 0xbb, 0xbf)));
t("bom twice", () => d.decode(bytes(0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf, 0x41)));
t("bom ignored", () => new TextDecoder("utf-8", { ignoreBOM: true }).decode(bytes(0xef, 0xbb, 0xbf, 0x41)));
t("bom split", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(0xef, 0xbb), { stream: true }), d.decode(bytes(0xbf, 0x41), { stream: true }), d.decode(bytes(0xef, 0xbb, 0xbf))].map(hex);
});
t("bom after empty chunk", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(), { stream: true }), d.decode(bytes(0xef, 0xbb, 0xbf, 0x41))].map(hex);
});
t("bom per stream", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(0xef, 0xbb, 0xbf, 0x41)), d.decode(bytes(0xef, 0xbb, 0xbf, 0x42))].map(hex);
});
t("bom after ascii chunk", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(0x41), { stream: true }), d.decode(bytes(0xef, 0xbb, 0xbf)), d.decode(bytes(0xef, 0xbb, 0xbf))].map(hex);
});
t("invalid", () => d.decode(bytes(0x41, 0x80, 0xc0, 0xc2, 0x41, 0xe0, 0x80, 0xe0, 0xa0, 0x41, 0xed, 0xa0, 0x80, 0xf0, 0x90, 0x80, 0xf4, 0x90, 0xff, 0xf0, 0x90, 0x80)));
t("invalid overlong and surrogates", () => d.decode(bytes(0xc1, 0xbf, 0xe0, 0x9f, 0xbf, 0xed, 0xbf, 0xbf, 0xf0, 0x8f, 0xbf, 0xbf, 0xf4, 0x90, 0x80, 0x80, 0xf8, 0x88)));
t("max code points", () => d.decode(bytes(0x7f, 0xdf, 0xbf, 0xef, 0xbf, 0xbf, 0xf4, 0x8f, 0xbf, 0xbf, 0xc2, 0x80)));
t("latin1 range", () => d.decode(bytes(0xc3, 0xa9, 0xc3, 0xbf, 0x41)));
t("fatal", () => new TextDecoder("utf-8", { fatal: true }).decode(bytes(0x41, 0x80)));
t("fatal truncated", () => new TextDecoder("utf-8", { fatal: true }).decode(bytes(0x41, 0xe2, 0x82)));
t("fatal truncated stream", () => new TextDecoder("utf-8", { fatal: true }).decode(bytes(0x41, 0xe2, 0x82), { stream: true }));
t("fatal valid", () => new TextDecoder("utf-8", { fatal: true }).decode(bytes(0xef, 0xbb, 0xbf, 0xe2, 0x82, 0xac)));
t("fatal resets", () => {
  const d = new TextDecoder("utf-8", { fatal: true });
  d.decode(bytes(0xe2), { stream: true });
  try {
    d.decode(bytes(0x41), { stream: true });
  } catch (e) {}
  return d.decode(bytes(0x82, 0x41));
});
t("fatal resets held bytes", () => {
  const d = new TextDecoder("utf-8", { fatal: true });
  try {
    d.decode(bytes(0x80, 0xe2), { stream: true });
  } catch (e) {}
  return d.decode(bytes(0x82, 0xac));
});
t("fatal then fine", () => {
  const d = new TextDecoder("utf-8", { fatal: true });
  try {
    d.decode(bytes(0x80));
  } catch (e) {}
  return d.decode(bytes(0xef, 0xbb, 0xbf, 0x41));
});
t("stream byte by byte", () => {
  const d = new TextDecoder();
  const r = [];
  for (const b of [0xf0, 0x9f, 0x98, 0x80, 0xe2, 0x82, 0xac, 0x41]) r.push(d.decode(bytes(b), { stream: true }));
  r.push(d.decode());
  return r.map(hex);
});
t("stream truncated at end", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(0xf0, 0x9f), { stream: true }), d.decode()].map(hex);
});
t("stream broken sequence", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(0xf0, 0x9f), { stream: true }), d.decode(bytes(0x41))].map(hex);
});
t("stream invalid tail", () => {
  const d = new TextDecoder();
  return [d.decode(bytes(0x41, 0xe0, 0x80), { stream: true }), d.decode(bytes(0xf4, 0x90), { stream: true }), d.decode(bytes(0xed), { stream: true }), d.decode(bytes(0xa0))].map(hex);
});
t("stream option getter", () => {
  const d = new TextDecoder();
  const log = [];
  const a = d.decode(bytes(0xe2), { get stream() { log.push("stream"); return "yes"; } });
  return [hex(a), hex(d.decode(bytes(0x82, 0xac))), log];
});

// Random bytes, whole and in random pieces, and random strings with lone surrogates.
let seed = 12345;
const rnd = (n) => {
  seed = (seed * 1103515245 + 12345) >>> 0;
  return (seed >>> 8) % n;
};
const pick = [0x41, 0x7f, 0x80, 0xbf, 0xc0, 0xc2, 0xdf, 0xe0, 0xa0, 0xed, 0x9f, 0xef, 0xbb, 0xf0, 0x90, 0xf4, 0x8f, 0xf5, 0xff, 0x00, 0xe2, 0x82, 0xac];
let h = 0;
const mix = (s) => {
  for (let i = 0; i < s.length; i++) h = (Math.imul(h, 31) + s.charCodeAt(i)) | 0;
  h = (Math.imul(h, 31) + 7) | 0;
};
for (let n = 0; n < 400; n++) {
  const len = rnd(12);
  const b = new Uint8Array(len);
  for (let i = 0; i < len; i++) b[i] = rnd(3) ? pick[rnd(pick.length)] : rnd(256);
  mix(new TextDecoder().decode(b));
  const sd = new TextDecoder("utf-8", { ignoreBOM: rnd(2) === 1 });
  let s = "";
  for (let i = 0; i < len; ) {
    const k = 1 + rnd(4);
    s += sd.decode(b.subarray(i, i + k), { stream: true });
    i += k;
  }
  mix(s + sd.decode());
  try {
    mix(new TextDecoder("utf-8", { fatal: true }).decode(b));
  } catch (e) {
    mix(e.name);
  }
  let str = "";
  for (let i = rnd(8); i > 0; i--) str += ["\ud83d", "\ude00", "a", "\u00e9", "\u20ac", "\ud83d\ude00"][rnd(6)];
  mix(Array.from(new TextEncoder().encode(str)).join());
  const u = new Uint8Array(rnd(10));
  const r = new TextEncoder().encodeInto(str, u);
  mix(r.read + "," + r.written + ":" + u.join());
}
out.push("random " + h);

// encode and encodeInto.
const e = new TextEncoder();
t("encode", () => Array.from(e.encode("A\u20ac\ud83d\ude00\ud800x\udc00")));
t("encode none", () => Array.from(e.encode()));
t("encode undefined", () => Array.from(e.encode(undefined)));
t("encode null", () => Array.from(e.encode(null)));
t("encode number", () => Array.from(e.encode(12)));
t("encode object", () => Array.from(e.encode({ toString: () => "\u00ff" })));
t("encode symbol", () => e.encode(Symbol()));
t("encode result", () => {
  const u = e.encode("ab");
  return [Object.prototype.toString.call(u), u.byteOffset, u.buffer.byteLength, Object.getPrototypeOf(u) === Uint8Array.prototype];
});
t("encode rope", () => {
  const a = "ab" + String.fromCharCode(0xd83d);
  return Array.from(e.encode(a + String.fromCharCode(0xde00) + "c"));
});
t("encode lone at end", () => Array.from(e.encode("x\ud800")));
t("encode large", () => {
  const big = "x".repeat(70000) + "\u20ac" + "\u00e9".repeat(3);
  const u = e.encode(big);
  return [u.length, u[70000], u[70005], new TextDecoder().decode(u) === big];
});
t("encodeInto", () => {
  const u = new Uint8Array(5);
  const r = e.encodeInto("A\u20ac\ud83d\ude00", u);
  return [r, Array.from(u), Object.keys(r), Object.getPrototypeOf(r) === Object.prototype];
});
t("encodeInto lone surrogates", () => {
  const u = new Uint8Array(8);
  return [e.encodeInto("\ud800\udc00\udbff", u), Array.from(u)];
});
t("encodeInto lone does not fit", () => {
  const u = new Uint8Array(5);
  return [e.encodeInto("a\ud800\ud800", u), Array.from(u)];
});
t("encodeInto pair does not fit", () => {
  const u = new Uint8Array(3);
  return [e.encodeInto("\ud83d\ude00", u), Array.from(u)];
});
t("encodeInto stops at the first misfit", () => {
  const u = new Uint8Array(4);
  return [e.encodeInto("\u20ac\u20aca", u), Array.from(u)];
});
t("encodeInto latin1", () => {
  const u = new Uint8Array(3);
  return [e.encodeInto("a\u00e9\u00e9", u), Array.from(u)];
});
t("encodeInto subarray", () => {
  const b = new Uint8Array(6);
  return [e.encodeInto("abcdef", b.subarray(2, 4)), Array.from(b)];
});
t("encodeInto empty", () => e.encodeInto("abc", new Uint8Array(0)));
t("encodeInto one argument", () => e.encodeInto("a"));
t("encodeInto detached", () => {
  const b = new ArrayBuffer(4);
  const u = new Uint8Array(b);
  b.transfer();
  return e.encodeInto("ab", u);
});
t("encodeInto clamped", () => e.encodeInto("ab", new Uint8ClampedArray(4)));
t("encodeInto Uint16Array", () => e.encodeInto("ab", new Uint16Array(4)));
t("encodeInto array", () => e.encodeInto("ab", [0, 0]));
out.join("\n");
