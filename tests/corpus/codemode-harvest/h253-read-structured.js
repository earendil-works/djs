// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}east zeta\n  return item test ready zeta\n  return south\nconst theta kappa\nconst west red west blue\n  if (beta omega\n  if (blue theta\nexport beta\n  if (record north beta\nfunction omega status south\n  if (gamma east value test\nfunction item north\nlet blue delta south\nfunction delta gamma delta eta"
  },
  {
   "ok": true,
   "value": "  if (south item\nfunction status red record omega\nconst test build alpha\n}lambda\nexport north item blue west\n  return record beta south south\nlet kappa eta\nfunction kappa done beta value\nfunction eta\nconst theta\n  if (iota epsilon value"
  }
 ],
 "les__blender__execute_blender_code": [
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
     "build": {
      "value": "zeta ready"
     },
     "test": {
      "value": "check epsilon"
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
text(await tools.read({path:"qod1/punu2.py",offset:1,limit:null}));
text(await tools.read({path:"qod3/qan/qod4/punu5.js",offset:1,limit:110}));
text(await tools.les__blender__execute_blender_code({code:"vinase nir\nfepisa={'vevul':nir.qade.is_vevul,'pokevap':doku1.filerus.kul.pokevap_runevi, 'tufitivob':[(m.qote,zuge(m.futones_faruk)) qon m in nir.qade.tufitivob][:12]}"}));
})(fake);
