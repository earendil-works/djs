// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "fup__blender__execute_blender_code": [
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
     "gamma": {
      "value": "alpha value"
     },
     "value": {
      "value": "south ready"
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
const r = await tools.fup__blender__execute_blender_code({code:"damope niv\nsunut=niv.zapized.sunut\nqev mizeb in tosa(niv.koro.fodovu):\n    if mizeb!=sunut qez mizeb.kano.kenemivopu('DemaMotagire') qez mizeb.masoki is Veme:\n        niv.koro.fodovu.fufoso(mizeb)\nsunut.kano='DemaMotagire'\nniv.tik.wm.gato_as_leluruza(betifofa='/pudi/pisu/qas1/qas2/qas3/didi4.karug')\npagovu={'sunut':sunut.kano,'fodovu':[(s.kano,fup(s.buripib)) qev s in niv.koro.fodovu]}"});text(r.structuredContent?.result||r);
})(fake);
