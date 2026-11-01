// String.prototype (0034), except the regex methods (0033).
function attempt(label, fn) {
  try {
    console.log(label, "->", fn());
  } catch (e) {
    console.log(label, "->", String(e));
  }
}
const s = "Hello, World";
console.log(s.at(-1), s.charAt(4), s.charCodeAt(1), s.codePointAt(0), "😀".codePointAt(0), "😀".codePointAt(1), s.charAt(99) === "");
console.log(s.indexOf("o"), s.indexOf("o", 5), s.lastIndexOf("o"), s.lastIndexOf("o", 5), s.includes("World"), s.startsWith("Hell"), s.endsWith("World"), s.endsWith("Hello", 5));
console.log(s.slice(-5), s.slice(2, 4), s.substring(7), s.substring(5, 0), s.substr(-5, 3), s.concat("!", 1, null));
console.log("ab".repeat(3), "x".padStart(4, "ab"), "x".padEnd(3), "abc".padStart(2), "5".padStart(3, "0"));
console.log("Straße".toUpperCase(), "ΑΣ ΑΣ.".toLowerCase(), "İ".toLowerCase().length, "ǅ".toUpperCase(), "MiXeD".toLowerCase(), "ﬀ".toUpperCase());
console.log("[" + "  \t pad \n ".trim() + "]", "[" + "  x ".trimStart() + "]", "[" + " x  ".trimEnd() + "]", "\u00a0\ufeffy".trim());
console.log("a\ud800".isWellFormed(), "ok".isWellFormed(), "a\ud800b".toWellFormed().charCodeAt(1));
console.log("a".localeCompare("b"), "b".localeCompare("a"), "a".localeCompare("a"), ["b", "a", "C"].sort((x, y) => x.localeCompare(y)));
console.log("plain".normalize(), "café".normalize("NFC"));
console.log(String.fromCharCode(72, 105, 0x1F600 & 0xffff), String.fromCodePoint(0x1F600, 65).length, String.raw({ raw: ["a", "b", "c"] }, 1, 2));
console.log(String.prototype.trimLeft === String.prototype.trimStart, new String("abc").length, Object.keys(new String("ab")));
let big = "";
for (let i = 0; i < 100; i++) big += "line " + i + "\n";
console.log(big.length, big.indexOf("line 99"), big.lastIndexOf("line 1\n"), big.slice(0, 7).trim().toUpperCase());
attempt("repeat -1", () => "a".repeat(-1));
attempt("repeat Infinity", () => "a".repeat(Infinity));
attempt("fromCodePoint", () => String.fromCodePoint(0x110000));
attempt("normalize bad", () => "a".normalize("X"));
attempt("on null", () => String.prototype.trim.call(null));
attempt("startsWith regexp-like", () => "abc".startsWith({ [Symbol.match]: true }));
