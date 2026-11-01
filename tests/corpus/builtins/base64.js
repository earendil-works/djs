// Uint8Array base64/hex (0039): codemode moves images as base64.
// difftest: no-stress 7 s suspended, 13 s under GC stress (debug, 0053)
function q(s) {
  let out = "'";
  for (const c of s) out += c === "\n" ? "\\n" : c === "\t" ? "\\t" : c;
  return out + "'";
}
function show(f) {
  try {
    const r = f();
    return r instanceof Uint8Array ? "[" + Array.from(r).join(",") + "]" : typeof r === "object" ? "{read:" + r.read + ",written:" + r.written + "}" : String(r);
  } catch (e) {
    return e.name + ": " + e.message;
  }
}
const inputs = ["", "aGk=", "aGk", "aG", "a", "aGkh", " a G k h ", "aGk=x", "aG==", "aGk==", "-_-_", "+/+/", "AA==", "AB==", "AAA=", "AAB=", "aGkhZm9v", "aGkhZm9", "aGkhZm", "aG\tk\nh", "aGk =", "=", "aGkh=", "aGkhZg==", "aGkhZg"];
const handlings = [undefined, "strict", "stop-before-partial"];
for (const s of inputs) {
  const line = [q(s)];
  for (const h of handlings) {
    line.push(show(() => Uint8Array.fromBase64(s, h ? { lastChunkHandling: h } : undefined)));
    line.push(show(() => Uint8Array.fromBase64(s, { alphabet: "base64url", lastChunkHandling: h })));
  }
  for (const size of [0, 1, 2, 3, 5]) {
    const t = new Uint8Array(size);
    line.push(show(() => t.setFromBase64(s, { lastChunkHandling: h0(size) })) + "=" + Array.from(t).join(","));
  }
  console.log(line.join(" | "));
}
function h0(n) {
  return handlings[n % 3];
}
for (const s of ["", "00ff", "0", "zz", "ABcd", "a b", "0102030405"]) {
  const t = new Uint8Array(3);
  console.log(q(s), show(() => Uint8Array.fromHex(s)), show(() => t.setFromHex(s)), Array.from(t).join(","));
}
// A binary blob round trip, as an image would travel.
const blob = new Uint8Array(1000);
let x = 12345;
for (let i = 0; i < blob.length; i++) {
  x = (x * 1103515245 + 12345) % 2147483648;
  blob[i] = x & 255;
}
const b64 = blob.toBase64();
const back = Uint8Array.fromBase64(b64);
let same = back.length === blob.length;
for (let i = 0; i < blob.length; i++) if (back[i] !== blob[i]) same = false;
console.log(b64.length, b64.slice(0, 40), same, blob.toHex().slice(0, 32), Uint8Array.fromHex(blob.toHex()).length);
console.log(blob.subarray(10, 20).toBase64({ alphabet: "base64url", omitPadding: true }));
