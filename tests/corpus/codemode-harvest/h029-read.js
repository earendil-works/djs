// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "const red red west record\nconst green west value lambda\n}gamma record epsilon"
  },
  {
   "ok": true,
   "value": "}omega kappa\nlet done west build\nexport south alpha delta\nconst omega omega ready\nexport green omega\nlet north red\nconst ready item\nconst status record kappa\n}ready test alpha\n  return lambda entry east\nlet delta item red theta\n  return zeta\n}delta"
  },
  {
   "ok": true,
   "value": "export record omega record east\nexport epsilon kappa\n}status status lambda ready\nlet done west check\nlet iota east value\nfunction green\n}record\n  if (gamma\nlet alpha record\nfunction west"
  },
  {
   "ok": true,
   "value": "  return north epsilon\nconst beta\n  if (beta check delta eta\n  if (red item zeta\n}delta entry item build\nlet iota status\nconst south kappa alpha\n  if (east\nlet eta status\n  return west build\n  if (entry iota\nfunction build done check"
  },
  {
   "ok": true,
   "value": "  return check south kappa\nexport entry green\n}delta\nlet build entry check\nconst red green theta red\nconst west lambda test test\n  return record done\nfunction red south\nconst build test"
  },
  {
   "ok": true,
   "value": "function gamma\nconst eta beta done gamma\nfunction alpha check theta\n  return epsilon\n  if (north iota zeta\nlet north gamma\nconst south iota item\n  if (test\nfunction eta gamma item blue\nfunction item check test\nlet alpha"
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
const base = "/home/user/dir1/dir2/";
const files = ["packages/dir3/src/dir4/dir5/file6.ts","packages/dir3/src/dir4/file7.ts","packages/dir3/src/dir4/file8.ts","packages/dir3/src/file9.ts","packages/dir10/src/core/file6.ts","packages/dir3/src/dir11/file12.ts"];
const r = await Promise.allSettled(files.map(p=>tools.read({path:base+p, offset:null, limit:250})));
return r.map((x,i)=>`#### ${files[i]}\n`+(x.value??x.reason)).join("\n");
})(fake);
