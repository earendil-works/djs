// A regex literal right after a closed block or statement head, inside `try` (0217):
// the finally lookahead must lex it as a regex, not as division.
const r = [];
try { if (1) {} r.push(/x./.test("xy")); } finally { r.push("f1"); }
try { if (1) {} /x./.test("xy") && r.push("if"); } finally {}
try { if (0) {} else {} /x./g.test("xz") && r.push("else"); } finally {}
try { {} /'/.test("'") && r.push("block"); } finally {}
try { for (let i = 0; i < 1; i++) {} /x./.test("xy") && r.push("for"); } finally {}
try { let n = 0; while (n++ < 1) {} /x./.test("xy") && r.push("while"); } finally {}
try { L: { break L; } /x./.test("xy") && r.push("label"); } finally {}
try { switch (1) { case 1: {} /x./.test("xy") && r.push("case"); } } finally {}
try { function f() {} /x./.test("xy") && r.push("function"); } finally {}
try { class A { m() {} } /x./.test("xy") && r.push("class"); } finally {}
try { if (r) /x./.test("xy") && r.push("if-head"); } finally {}
try { try {} finally {} /x./.test("xy") && r.push("nested"); } finally {}
try { throw 1; } catch { if (1) {} /x./.test("xy") && r.push("catch"); } finally {}
try { throw 1; } catch (e) { if (1) {} /'/.test("'") && r.push("catch-param"); } finally {}
try {} finally { if (1) {} /x./.test("xy") && r.push("finally"); }
try { if (1) {}
/./s.source === "." && r.push("newline"); } finally {}
for (const x of [1]) { try { if (x) {} /x./.test("xy") && r.push("loop"); } finally { continue; } }
// After an object literal, a call or a template it stays division.
let d = 6;
try { d /= {valueOf() { return 2; }} / 1; r.push(d); } finally {}
try { r.push({valueOf() { return 8; }} / 2 / 2); } finally {}
try { r.push(Math.max(4) / 2); } finally {}
try { r.push(`${{}}`.length / 3); } finally {}
console.log(r.join(" "));
