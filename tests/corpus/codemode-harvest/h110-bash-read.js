// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export delta\n  return entry build east\n  if (value omega epsilon eta\n}lambda south\n  if (omega zeta\n  if (gamma zeta\nexport west eta"
  },
  {
   "ok": true,
   "value": "const beta\nlet done west\nfunction delta\nexport entry test check\nconst ready blue\n}delta\nlet item east alpha record\n  return theta value epsilon\nexport east check\nexport south omega value\n  if (test delta item\nconst done test red test\nexport east done item"
  },
  {
   "ok": true,
   "value": "let blue lambda green delta\nlet theta\n}alpha theta build east\nexport epsilon record\nfunction beta epsilon red\n  return item epsilon"
  },
  {
   "ok": true,
   "value": "let theta\nconst gamma\n}iota item gamma\n}entry\n}test\nlet beta iota"
  },
  {
   "ok": true,
   "value": "  if (south beta north\nfunction eta lambda\nexport gamma theta build"
  },
  {
   "ok": true,
   "value": "  if (build omega north\nfunction record north\nconst zeta eta\n}green green test eta\n}alpha done green\nlet green ready lambda done\nfunction record delta alpha eta\nfunction theta lambda done south\n  return south entry\n}omega\n}zeta north item beta"
  },
  {
   "ok": true,
   "value": "epsilon check alpha build east beta beta\nbuild iota build\nred status kappa red ready theta theta\nlambda record theta item eta 8002"
  },
  {
   "ok": true,
   "value": "## Red iota alpha build value\n## Record entry gamma value iota east blue beta\n## Epsilon south blue delta entry check green record\n1. Build ready done epsilon\n- Item kappa item zeta check beta lambda north\n- Item epsilon ready test iota entry beta\n# Alpha build\n## Build item check epsilon check\nValue zeta theta delta west\n## Check entry record ready"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file5.ts:141: check test lambda iota green\nsrc/file6.ts:360: build entry\nsrc/file9.ts:353: lambda blue blue west status\nsrc/file4.ts:81: west build east alpha\nsrc/file4.ts:310: gamma entry\nsrc/file1.ts:266: epsilon test item alpha gamma delta\nsrc/file1.ts:56: south value east ready west done\nsrc/file6.ts:299: alpha test iota kappa\nsrc/file4.ts:294: done beta record red\nsrc/file4.ts:237: test gamma east epsilon blue east\nsrc/file5.ts:217: test north item zeta zeta green\nsrc/file5.ts:126: ready gamma\nsrc/file2.ts:356: blue delta\nsrc/file4.ts:226: beta omega"
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
for (const path of ['dir1/src/core/file2.js','dir1/src/dir3/file4.js','dir1/src/dir3/file5.js','dir1/src/dir6/file7.js','dir1/tests/file8.js','dir1/file9.py','dir1/file10.html','/home/user/dir11/dir12/dir13/dir14/file15.md']) text({path,content:await tools.read({path,offset:path.endsWith('pimeken-somezo.js')?235:undefined})});
text(await tools.bash({command:"ls -la .pi; zodinid -v vabe; zodinid -v uv; pozi -nP -iNOV -sNOV:DUBUZE | qose -E '8000|8080|9222|5173' || fopa"}));
})(fake);
