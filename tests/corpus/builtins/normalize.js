// String.prototype.normalize in all four forms, and localeCompare's NFC step.
const hex = (s) => Array.from(s, (c) => c.codePointAt(0).toString(16)).join(" ");
const samples = [
  "abc",
  "caf\u00e9",
  "cafe\u0301",
  "\u212b\u2126",
  "\u1e9b\u0323",
  "a\u0307\u0323",
  "\u0071\u0307\u0323",
  "\ufb01\u2460\u00bd\u3300",
  "\uac00\uac01\u1100\u1161\u11a8",
  "\u0f73\u0f75\u0f81",
  "\u0344\u0958",
  "x\ud800y\udc00z",
  "\ud834\udd5e\ud834\udd60",
  "\u1100\uac00\u11a8",
  "\u00c5\u030a\u0316",
];
for (const s of samples) {
  console.log(hex(s), "|", ["NFC", "NFD", "NFKC", "NFKD"].map((f) => hex(s.normalize(f))).join(" | "));
}
console.log("\u00e9".normalize() === "\u00e9", "e\u0301".normalize() === "\u00e9");
try {
  "x".normalize("nfc");
} catch (e) {
  console.log(e.constructor.name);
}
console.log("\u00e9".localeCompare("e\u0301"), "\u212b".localeCompare("\u00c5"), "a".localeCompare("\u00e1") < 0);
const long = "e\u0301".repeat(2000);
console.log(long.normalize().length, long.normalize("NFD").length, long.normalize().normalize("NFD") === long);
