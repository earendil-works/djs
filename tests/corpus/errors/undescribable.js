// When even describing the error throws, a placeholder is shown.
const e = new Error("x");
Object.defineProperty(e, "name", { get() { throw new Error("no name"); } });
throw e;
