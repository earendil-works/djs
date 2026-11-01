// Messages of common ReferenceErrors.
function attempt(label, fn) {
  try {
    fn();
    console.log(label, "-> no error");
  } catch (e) {
    console.log(label, "->", e);
  }
}
attempt("undefined variable", () => missingName);
attempt("undefined variable call", () => missingFunction());
attempt("typeof undefined variable", () => console.log(typeof missingName));
attempt("let before declaration", () => { value; let value = 1; });
attempt("const before declaration", () => { value; const value = 1; });
attempt("class before declaration", () => { new K(); class K {} });
attempt("this before super", () => { class A {} class B extends A { constructor() { this.x = 1; super(); } } new B(); });
attempt("assign to undeclared", () => { undeclared = 1; });
attempt("closure over TDZ", () => { const f = () => later; f(); let later = 1; });
