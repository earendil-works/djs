// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}epsilon\nconst entry omega red delta\n  return lambda item done\nexport red beta record kappa\nlet red iota omega\n  return kappa item item eta\nlet gamma"
  },
  {
   "ok": true,
   "value": "function entry\n  return alpha entry red red\nfunction ready\nlet ready\nfunction test\n  if (north check record\nfunction gamma ready green kappa\nexport east west green beta\n  return green epsilon east west\nfunction status\n  return status item delta\nlet value south item ready\n  if (zeta green item green\n  if (east"
  },
  {
   "ok": true,
   "value": "## Blue record north entry alpha entry\n# Kappa done\n1. Record lambda green iota done\n# Value build eta record ready\n## Omega omega epsilon\n## Beta lambda value check green build delta entry\n## Record theta value north record\n# Status record south ready west ready north record\n# Gamma check ready\n## Lambda item red value\n1. Record kappa epsilon delta eta record south south\n# Eta item green beta test west theta check"
  },
  {
   "ok": true,
   "value": "## Green test\n- Ready gamma gamma\n1. Entry south entry entry entry"
  },
  {
   "ok": true,
   "value": "export status gamma value\nfunction theta north\nlet zeta omega record check\n}item record status\nfunction blue entry record"
  }
 ],
 "qap__blender__execute_blender_code": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "value": {
      "value": "status"
     },
     "theta": {
      "value": "east epsilon south"
     }
    }
   }
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
await Promise.allSettled(["dir1/file2.py","dir1/file3.py","dir4/docs/file5.md","dir1/file6.nis","dir4/src/file7.js"].map(async path => text({path,content:await tools.read({path,offset:1,limit:path.endsWith("setu.js")?220:null})})));
text(await tools.qap__blender__execute_blender_code({code:"vevata suk\nnebive = {'gafo': suk.bapa.mamugebe, 'rabemov': [(o.bele,o.fuvi) qin o in suk.vetibug.zolil.rabemov][:60]}"}));
})(fake);
