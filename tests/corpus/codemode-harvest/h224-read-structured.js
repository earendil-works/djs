// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let east south\n}south epsilon green\nconst beta ready\n  if (beta\n  return alpha\nconst done"
  },
  {
   "ok": true,
   "value": "}lambda north item entry\nlet status build\n  return west record record delta\n  if (test\nfunction beta"
  },
  {
   "ok": true,
   "value": "export south red iota\nlet value\nconst delta build test south\n}build east omega beta\nexport ready omega\n  if (north\nconst north kappa lambda\nconst blue east build\nlet gamma\n}green"
  },
  {
   "ok": true,
   "value": "const green\n  return entry epsilon\nexport item green entry\n}check done west\n}south\nfunction item\nfunction done kappa alpha done\n  if (kappa ready value north\nlet test eta status"
  },
  {
   "ok": true,
   "value": "function value record theta\nexport test\nfunction zeta beta\nconst green omega build theta"
  },
  {
   "ok": true,
   "value": "}gamma\n}red ready omega west\n  return blue"
  }
 ],
 "qig__blender__execute_blender_code": [
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
     "delta": {
      "value": "alpha omega"
     },
     "lambda": {
      "value": "entry"
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
await Promise.allSettled(["dir1/src/dir2/file3.js","dir1/src/dir2/file4.js","dir1/src/dir5/file6.js","dir1/file7.py","dir1/src/dir8/file9.js","dir1/src/dir10/file11.js"].map(async path=>text({path,content:await tools.read({path,offset:1,limit:null})})));
text(await tools.qig__blender__execute_blender_code({code:"vanilo qeb\ndobonu={'qure':qeb.soba.lizobovi,'goten':qeb.soba.is_goten,'tobov':qeb.gevubip.tobov.duvu}"}));
})(fake);
