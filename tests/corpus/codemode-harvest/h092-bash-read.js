// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}check blue delta\nexport test blue\nexport zeta theta south red\nfunction alpha west theta west\nfunction east kappa entry\nlet west"
  },
  {
   "ok": true,
   "value": "let iota\nexport test status\n}value theta build"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "east west blue item north entry red west\nstatus south gamma gamma epsilon zeta build 8197\neast build gamma west south red south\ncheck omega blue\niota build south status entry record north\neast entry value red iota theta value gamma"
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
text(await tools.read({path:"midukala/qop1/qad/qop2/qop3/ropi4.ts",offset:270,limit:225}));
text(await tools.bash({command:"rg -n 'zana_duvebitab_sovam|dipuZanaFosa|rikuluZanaFosa|zana_fosa' midukala/qop5/qad midukala/qop1/qad/zure/qop2/ropi6.ts midukala/qop1/qad/zure/ropi7.ts | zuvo -65; rg -n 'kinadire rabuRokarovagSusugef|qadBanovuk|pidekMs' midukala/qop8/qad/qop9/ropi10.ts; rg -n 'ramodidZana\\(' midukala/qop1/nepo/qop11/ropi12.ts | zuvo",timeout:30}));
text(await tools.read({path:"midukala/qop1/qad/qop2/qop3/ropi13.ts",offset:1,limit:210}));
})(fake);
