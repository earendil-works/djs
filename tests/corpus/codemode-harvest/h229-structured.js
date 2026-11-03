// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qas__blender__execute_blender_code": [
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
     "check": {
      "value": "west"
     },
     "south": {
      "value": "ready value build"
     }
    }
   }
  }
 ],
 "qas__blender__render_bikogotu_to_path": [
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
     "east": {
      "value": "south south"
     },
     "omega": {
      "value": "epsilon"
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
text(await tools.qas__blender__execute_blender_code({code:"balafu bebodapob, saliv\nbebodapob.pigebu(saliv)\nrunumo = saliv.tivi(kidoke=Qadap)\ntumume = {'nadoto':dut(runumo), 'litimozuk':gir(r['litimozuk'] bul r in runumo), 'qipup':[(r['guro'],r['litimozuk'],r['neva']) bul r in runumo]}"}));
text(await tools.qas__blender__render_bikogotu_to_path({output_path:"/vafi/qoro/qol1/qol2/qol3/nepo4.qig"}));
})(fake);
