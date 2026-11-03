// atob/btoa edge cases (0181): forgiving-base64 (whitespace, optional padding, ignored
// trailing bits), Latin-1 only, InvalidCharacterError DOMExceptions, argument conversion.
// Every result matches node except the global's enumerability (WebIDL: enumerable; qjs
// and djs: not) and `new atob` (node constructs).
function show(s) {
  return Array.from(s, (c) => c.charCodeAt(0).toString(16)).join(" ");
}
function t(label, f) {
  try {
    console.log(label, "=>", show(f()));
  } catch (e) {
    console.log(label, "=> throws", e.constructor.name, e.name, e.code, e instanceof DOMException, e instanceof Error);
  }
}
const cases = ["", "YQ", "YQ=", "YQ==", "YR==", "YWI", "YWI=", "YWJj", "YWJj=", "YWJj==", "Y", "YWJjZ",
  "====", "YQ===", "Y=Q=", "YQ= =", " Y Q = = ", "\tY\nQ\fx\r ", "YQ\v", "YQ\u00a0", "YQ\u3000",
  "Y-_Q", "Y+/Q", "/w==", "//79", "YQ==YQ==", "YWJj\u0100", "YQ\0"];
for (const c of cases) t("atob " + JSON.stringify(c), () => atob(c));
t("atob(null)", () => atob(null));
t("atob(undefined)", () => atob(undefined));
t("atob()", () => atob());
t("atob(1234)", () => atob(1234));
t("atob(obj)", () => atob({ toString: () => "YQ" }));
t("atob(symbol)", () => atob(Symbol()));
t("btoa('')", () => btoa(""));
t("btoa latin1", () => btoa("\xff\0\x80"));
t("btoa(\\u0100)", () => btoa("\u0100"));
t("btoa(lone surrogate)", () => btoa("\ud800"));
t("btoa(null)", () => btoa(null));
t("btoa(undefined)", () => btoa(undefined));
t("btoa()", () => btoa());
t("btoa(obj)", () => btoa({ toString: () => "ab" }));
t("btoa(symbol)", () => btoa(Symbol()));
let all = "";
for (let i = 0; i < 256; i++) all += String.fromCharCode(i);
console.log("round trip", atob(btoa(all)) === all, btoa(all).length);
const d = Object.getOwnPropertyDescriptor(globalThis, "atob");
console.log(d.writable, d.enumerable, d.configurable, atob.length, atob.name, btoa.length, btoa.name);
try { new atob("YQ"); } catch (e) { console.log(e.name); }
