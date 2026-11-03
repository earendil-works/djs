// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export alpha\nconst north done kappa epsilon\nfunction west epsilon kappa check"
  },
  {
   "ok": true,
   "value": "}entry beta gamma\n  if (test done gamma\n  return zeta theta omega value\n  return north build\n  return south entry red\nfunction green delta\nconst alpha omega iota eta\n  return west omega zeta\nfunction beta zeta east\nfunction test value south\n}ready delta done\nexport theta build\n  return theta\n}blue"
  },
  {
   "ok": true,
   "value": "}epsilon theta\nexport eta red item entry\n  if (epsilon west alpha\nconst green north green\n  if (west\n}zeta build test iota"
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
await Promise.allSettled(["../dir1/packages/dir2/src/index.ts","../dir1/packages/dir2/src/dir3/file4.ts","../dir1/packages/dir2/src/dir3/file5.ts"].map(async path=>text(await tools.read({path}))));
})(fake);
