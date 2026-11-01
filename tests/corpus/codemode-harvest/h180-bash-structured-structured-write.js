// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "zuv__soguri__tools_imageGeneration_run": [
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
     "images": [
      {
       "dataUrl": "check eta",
       "mediaType": "eta lambda"
      }
     ],
     "kind": "soguri#image_generation",
     "model": "check eta value",
     "prompt": "beta",
     "text": "lambda build"
    }
   }
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "delta delta theta 4423\ndone value done status delta entry delta done 8799\nalpha delta entry east 6212\nbeta west lambda lambda gamma status epsilon south 1454\nsouth beta omega alpha 1945",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "north iota\nlambda red eta beta east epsilon item\ngreen record\nkappa zeta epsilon\nkappa green record west iota 9364\nomega eta kappa 7489\nbeta value done green\nwest gamma value record alpha\ndone check gamma green done status 5047\neta epsilon north blue green iota\nbeta build omega\ngreen value\nnorth entry 4726\nepsilon west",
    "truncated": false
   }
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "red eta beta build beta\ngreen eta blue blue alpha red alpha\ndelta zeta\ndone kappa north\nrecord status theta check\nkappa north done south value delta west theta\nentry lambda value epsilon ready"
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
const r = await tools.zuv__soguri__tools_imageGeneration_run({ body: { prompt: "A puno vafiza logako tinato mizupoz on a tuge kotevez, tiv fufazot qora, gobe gefegak gibif, dosikuvagevimu" } });
const res = r.structuredContent ?? r;
const vab = res.images[0];
const [meta, b64] = vab.dataUrl.split(",");
const qup = vab.mediaType.split("/")[1].replace("jpeg", "jpg");
await tools.bash({ command: "pipoz -p /dok/vem1" });
await tools.write({ path: "/dok/vem1/rovo2.b64", content: b64 });
const out = await tools.bash({ command: `zipo64 -d -i /dok/vem1/rovo2.b64 -o /dok/vem1/vem3${qup} && rm /dok/vem1/rovo2.b64 && rovo /dok/vem1/vem3${qup}` });
return { model: res.model, mediaType: vab.mediaType, text: res.text, out: out.output };
})(fake);
