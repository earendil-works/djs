// The canonical dump shared with djs-core's tests (crates/djs-core/src/wire/tests.rs,
// `dump`): `undefined`, `null`, `true`, `n:<String(x)>` (`n:-0`), `s:"..."` with everything
// but printable ASCII as `\uXXXX`, `[a,<hole>]`, `{"k":v}` in property order.

export function quote(s: string): string {
  let out = '"';
  for (let i = 0; i < s.length; i++) {
    const u = s.charCodeAt(i);
    if (u >= 0x20 && u < 0x7f && u !== 0x22 && u !== 0x5c) out += s[i];
    else out += "\\u" + u.toString(16).padStart(4, "0");
  }
  return out + '"';
}

export function dump(v: unknown): string {
  if (v === undefined) return "undefined";
  if (v === null) return "null";
  if (typeof v === "boolean") return String(v);
  if (typeof v === "number") return Object.is(v, -0) ? "n:-0" : "n:" + String(v);
  if (typeof v === "string") return "s:" + quote(v);
  if (v instanceof Uint8Array) return "[" + Array.from(v, (b) => "n:" + b).join(",") + "]";
  if (Array.isArray(v)) {
    const parts: string[] = [];
    for (let i = 0; i < v.length; i++) parts.push(i in v ? dump(v[i]) : "<hole>");
    return "[" + parts.join(",") + "]";
  }
  const o = v as Record<string, unknown>;
  return "{" + Object.keys(o).map((k) => quote(k) + ":" + dump(o[k])).join(",") + "}";
}
