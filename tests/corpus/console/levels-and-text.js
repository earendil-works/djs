// All console levels write output items; text() formats primitives with String().
console.info("info", 1);
console.warn("warn", { w: true });
console.error("error", [1]);
console.debug("debug");
text("text string");
text(42);
text(-0);
text(NaN);
text(null);
text(undefined);
text({ a: [1, 2] });
text([NaN, "s"]);
text(true);
text(Symbol("t"));
text(() => 1);
try {
  const c = {};
  c.c = c;
  text(c);
} catch (e) {
  console.log("text threw:", e);
}
try {
  text(1n);
} catch (e) {
  console.log("text threw:", e);
}
