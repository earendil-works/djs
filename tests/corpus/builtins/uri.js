// encodeURI/decodeURI(+Component) and escape/unescape (0054): results and URIError
// messages. pi's quickjs-wasi gives the same as dquickts here.
const out = [];
const t = (label, f) => {
  try {
    out.push(label + " = " + JSON.stringify(f()));
  } catch (e) {
    out.push(label + " -> " + e.name + ": " + e.message);
  }
};
const fns = { encodeURI, encodeURIComponent, decodeURI, decodeURIComponent, escape, unescape };
const inputs = ["a b", "\u00e9", "\ud800", "\udc00x", "\ud800\udc00", "%E9", "%", "%4", "%zz", "%C3%A9",
  ";/?:@&=+$,#", "-_.!~*'()", "\udbff\udfff", "%F0%9F%98%80", "%C0%80", "%ED%A0%80", "%F8%80",
  "%E0%A4", "%E0%A4%A", "%25%23%3b", "%u0041%41%zz%u12", "@*_+-./~\u00e4\u1234", ""];
for (const s of inputs) for (const [name, f] of Object.entries(fns)) t(name + "(" + JSON.stringify(s) + ")", () => f(s));
t("ToString", () => encodeURIComponent({ toString() { return "x y"; } }));
t("undefined", () => [encodeURI(), decodeURI(), escape(), unescape()]);
t("lengths", () => Object.values(fns).map((f) => f.name + "/" + f.length));
t("long", () => decodeURIComponent(encodeURIComponent("\u00e9\ud83d\ude00".repeat(1000))).length);
console.log(out.join("\n"));
