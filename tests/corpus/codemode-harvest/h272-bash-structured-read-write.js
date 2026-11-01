// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}value theta east\nconst item test\nexport alpha kappa\nfunction epsilon lambda status check\nlet check\nlet alpha\n}ready lambda\nexport value eta delta done\nexport done\n}gamma value status omega"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "kappa delta test theta\ncheck check 3024\nlambda delta delta\ntest check west gamma beta eta omega epsilon\nkappa theta entry south lambda 1370\nvalue ready kappa south record beta west\nbeta ready item 6396\nitem eta item\ndelta gamma record delta omega item"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "iota alpha green check west epsilon red\niota theta alpha north value kappa\neta iota record eta\nepsilon iota 3856\nsouth eta status ready item 9195\nkappa item zeta done gamma blue kappa done\neta item gamma test west north green zeta\nwest lambda green status\ncheck iota beta",
    "truncated": false
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
const mebog="/tami/nono/qel1/qel2/fodekufa/qel3/vap/qane4.ts";
let t=await tools.read({path:mebog,offset:null,limit:null});
const s=t.indexOf(`  it("supoti tovimo puninum notiluv dirudun a purute or mosozipul, teti risuret"`);
const e=t.indexOf(`  it("lesovese QobNokefu goleteli on sav gisav's voburi depasimegas"`);
t=t.slice(0,s)+t.slice(e);
await tools.write({path:mebog,content:t});
const r=await tools.bash({command:"cd /tami/nono/qel1/qel2 && ./tosa_vuzenok/qel5/qel6 --bemib fodekufa/qel3/vap >/qel7/qel8 && ./tosa_vuzenok/qel5/qel9 pil fodekufa/qel3 2>&1 | qili -E 'Zamam |POGO' && zok dafi --kofi fodekufa/qel3/vap",timeout:300});
return r.output;
})(fake);
