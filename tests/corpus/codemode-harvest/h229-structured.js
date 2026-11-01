// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
text(await tools.qas__blender__execute_blender_code({code:"balafu bebodapob, saliv\nbebodapob.pigebu(saliv)\nrunumo = saliv.tivi(kidoke=Qadap)\ntumume = {'nadoto':dut(runumo), 'litimozuk':gir(r['litimozuk'] bul r in runumo), 'qipup':[(r['guro'],r['litimozuk'],r['neva']) bul r in runumo]}"}));
text(await tools.qas__blender__render_bikogotu_to_path({output_path:"/vafi/qoro/qol1/qol2/qol3/nepo4.qig"}));
})(fake);
