// Unicode property escapes (D1500): General_Category values and groups, Script and
// Script_Extensions, binary properties, properties of strings in v mode, negation,
// case-insensitive matching in u and v mode, and qjs's error messages.
const sample = "aZ5_ é\u00b2\u0663 Ωж\u4e2d\u3042\u30a2 😀👍🏽🇩🇪 \u200d\t\u2028";
const show = (re) => JSON.stringify(sample.match(re));
for (const p of ["L", "Lu", "Ll", "Letter", "gc=Lo", "General_Category=Decimal_Number", "N", "Nd", "digit", "P", "punct", "S", "So",
  "Z", "Zs", "C", "Cc", "Cf", "LC", "Script=Greek", "sc=Cyrl", "scx=Hira", "Script_Extensions=Kana", "Script=Han", "sc=Zyyy",
  "Alphabetic", "Alpha", "White_Space", "space", "Emoji", "Emoji_Presentation", "Extended_Pictographic", "ASCII", "ASCII_Hex_Digit",
  "Any", "Assigned", "ID_Start", "ID_Continue", "Uppercase", "Lowercase", "Join_Control", "Regional_Indicator", "Emoji_Modifier"]) {
  console.log(p, show(new RegExp("\\p{" + p + "}", "gu")), show(new RegExp("\\P{" + p + "}+", "gu")));
}
console.log(/\p{Lu}/iu.test("a"), /\P{Lu}/iu.test("A"), /\P{Lu}/iv.test("A"), /[^\P{Lu}]/iu.test("a"), /\p{Lowercase}/iu.test("A"), /[\p{Ll}--[a-z]]/v.test("q"));
console.log(/^\p{RGI_Emoji}$/v.test("👨‍👩‍👧"), /^\p{RGI_Emoji_Flag_Sequence}$/v.test("🇩🇪"), /^\p{Emoji_Keycap_Sequence}+$/v.test("1️⃣#️⃣"),
  /^\p{RGI_Emoji_Modifier_Sequence}$/v.test("👍🏽"), /^\p{Basic_Emoji}$/v.test("☺️"), /^[\p{RGI_Emoji}--\q{😀}]$/v.test("😀"));
console.log(JSON.stringify("x👨‍👩‍👧y🇩🇪z".match(/\p{RGI_Emoji}/gv)), JSON.stringify("Привет, мир! Hello".match(/\p{Script=Cyrillic}+/gu)));
console.log("café naïve ÆØÅ".replace(/\p{Lu}/gu, "_"), "3½ ٣ 𝟙".match(/\p{N}/gu).length, /^\p{Ideographic}+$/u.test("漢字"));
for (const [p, f] of [["\\p{Foo}", "u"], ["\\p{Script=Foo}", "u"], ["\\p{Script}", "u"], ["\\p{General_Category}", "u"], ["\\p{Foo=Latin}", "u"],
  ["\\p{RGI_Emoji}", "u"], ["\\P{RGI_Emoji}", "v"], ["[^\\p{RGI_Emoji}]", "v"], ["\\p{ASCII=Y}", "u"], ["\\p{lu}", "u"], ["\\p{WSpace}", "u"],
  ["\\p{Lu", "u"], ["\\p", "u"], ["\\p{gc=}", "u"], ["\\p{L=}", "u"]]) {
  try { new RegExp(p, f); console.log("ok", p, f); } catch (e) { console.log(p, f, e.name, e.message); }
}
console.log(/\p{L}/.test("p"), /\p{L}/.test("\\p{L}".slice(1)), "\\p".length);
