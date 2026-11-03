// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return iota ready green\n  if (green value delta omega\nexport eta\n}lambda beta test\n}check gamma ready eta\nconst east item\nconst south omega iota ready\nconst delta"
  },
  {
   "ok": true,
   "value": "  if (done check epsilon status\nlet done value done blue\n  return south eta"
  },
  {
   "ok": true,
   "value": "  return zeta\nlet check entry\n}east lambda entry red\n  if (build north item done"
  },
  {
   "ok": true,
   "value": "function green zeta\n  return epsilon green\nexport test delta build\n  return value"
  },
  {
   "ok": true,
   "value": "export north epsilon epsilon theta\n}lambda kappa lambda test\n  return eta theta\nlet theta test south delta\n  return delta\nlet west omega eta blue\nconst lambda\nconst build\nconst build"
  },
  {
   "ok": true,
   "value": "function green red done\nexport blue value zeta\n  if (blue green theta theta\nexport ready kappa delta theta\nconst north\nfunction delta item"
  },
  {
   "ok": true,
   "value": "  return delta ready status\nlet delta red\nfunction omega iota\nexport test status\n  if (north item value item\nexport red gamma\n  return entry north gamma\nconst north check\n  return blue done lambda\nlet omega kappa test gamma"
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
const files = ["packages/dir1/src/dir2/file3.ts","packages/dir1/src/file4.ts","packages/dir5/src/file6.ts","packages/dir5/src/file7.ts","packages/dir8/src/file9.ts","packages/dir8/src/file10.ts","packages/dir1/src/file11.ts"];
const r = await Promise.allSettled(files.map(p=>tools.read({path:p, offset:null, limit:null})));
return r.map((x,i)=>`#### ${files[i]}\n`+(x.value??x.reason)).join("\n");
})(fake);
