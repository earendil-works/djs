// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "epsilon delta west red\ngamma blue delta green alpha\neast item\ndone epsilon epsilon kappa iota\nkappa blue theta red build\nblue green item\ndone blue eta build beta south beta item"
  }
 ],
 "zez__blender__execute_blender_code": [
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
     "zeta": {
      "value": "blue alpha record"
     },
     "theta": {
      "value": "test check"
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
const stored = {
 "lokete": "ready gamma zeta zeta kappa record south status west test north east lambda record south omega green status item eta theta north status zeta build zeta iota eta done north iota zeta delta epsilon zeta"
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
await tools.bash({command: `cd /vono/bume/kuf1/kuf2 && zus -i '' 'kuf3/golo = z < 0.70 + gimo4.035/golo = z < 0.78 + 0.035/' kuf5/gimo6.py`});
const r = await tools.zez__blender__execute_blender_code({code: load('sevene')});
return JSON.stringify(r.structuredContent?.result ?? r).slice(0, 3000);
})(fake, fakeLoad);
