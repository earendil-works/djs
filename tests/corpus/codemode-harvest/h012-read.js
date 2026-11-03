// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}beta\nlet zeta\nconst test green west\nfunction epsilon epsilon test kappa\n  if (done west ready\nexport lambda value beta\n  return north kappa item\nlet record west record"
  },
  {
   "ok": true,
   "value": "let done build\n  return delta red\nlet entry\n}kappa beta theta record\nlet south beta build beta\nfunction ready epsilon record done\nexport eta gamma beta\n  return done red\n}omega eta\n  return blue east entry value\n  if (record kappa build alpha"
  },
  {
   "ok": true,
   "value": "function alpha status\nconst ready zeta\nconst beta done north west\nconst epsilon\nconst beta done red delta\nfunction ready\nlet zeta\nconst south west\nfunction iota\n}iota beta\nfunction check lambda"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
const ca = "packages/dir1/";
const [a, t1, t2] = await Promise.all([
  tools.read({path: ca + "ruz/geg2/geg3/tuva4.ts", offset: 4585, limit: 25}),
  tools.read({path: ca + "zaki/tuva5.ts", offset: null, limit: null}),
  tools.read({path: ca + "zaki/tuva6.ts", offset: null, limit: null}),
]);
text(a);

const names = s => s.split("\n").filter(l => /^\s*(describe|it|test)\(/.test(l)).join("\n");
text("GROUPS TEST:\n" + names(t1));
text("VAK TEST:\n" + names(t2));
})(fake);
