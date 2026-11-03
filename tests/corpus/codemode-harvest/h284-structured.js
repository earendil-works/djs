// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "det__blender__execute_blender_code": [
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
     "status": {
      "value": "item alpha north"
     },
     "south": {
      "value": "red test"
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
text(await tools.det__blender__execute_blender_code({code:"rafoge qam\nqam.qep.wm.nozo_rumanaki(nufogube='/tako/taba/quz1/quz2/quz3/furo4.vunag')\nvopufu = {'furo': qam.vadu.nufogube, 'zetolem': [o.gina duf o in qam.vadu.zetolem], 'futeler': [a.gina duf a in qam.vadu.futeler]}"}));
})(fake);
