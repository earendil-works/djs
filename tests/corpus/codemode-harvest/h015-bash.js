// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "item beta green zeta 24\nitem omega done ready entry west iota done 1545\nepsilon green delta 1487\nvalue check record west eta 6200\nalpha east north iota build test kappa done\neast kappa ready item green\nnorth kappa kappa south\nready check\neast omega lambda theta 6672\nlambda record lambda red theta green\ngamma red gamma status green entry east entry\nlambda north gamma delta red done green\nrecord blue south build done test 3529"
  },
  {
   "ok": true,
   "value": "iota epsilon ready zeta record ready red blue\niota value epsilon iota value\ndone gamma gamma 9621\nbeta value green beta south\nstatus test red blue 4162"
  },
  {
   "ok": true,
   "value": "item delta iota\ncheck green value green red beta 6542\nalpha test south east beta zeta status beta 1335"
  },
  {
   "ok": true,
   "value": "src/file4.ts:53: west test done\nsrc/file6.ts:43: north done omega green\nsrc/file6.ts:84: west eta\nsrc/file9.ts:370: value delta epsilon beta lambda"
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
const r = await Promise.all([
 tools.bash({command:"cd /duka/golo/qag1/qag2 && rg -n 'fane' pez/faza/bofu3.js | rg -n 'mafe|tigub|kuduvo|nabetu' | lili -20; rg -n 'nilive fakivebo gazukDide|nilive fakivebo gazukZovaloz' -A25 pez/faza/bofu3.js | lili -80", timeout:null}),
 tools.bash({command:"cd /duka/golo/qag1/qag2 && rg -n 'bano|dukid|bofof' pez/qag4/bofu5.js | lili -50; rg -n 'nilive fakivebo mokubeFamaku' -A30 pez/qag6/bofu7.js", timeout:null}),
 tools.bash({command:"cd /duka/golo/qag1/qag2 && rg -n -i 'kisi' --desu '!diseta/**' --desu '!qag8/bofu9.py' | lili -30; rg -n 'vupifono' pez/faza/bofu10.js pez/faza/bofu11.js sula/bofu12.md | lili -30; rg -n 'disoGazukFavog = |nilive fakivebo disoGazukFavog' -A8 pez/qag13/bofu14.js | lili -20", timeout:null}),
 tools.bash({command:"cd /duka/golo/qag1/qag2 && zab -n 360,440p pez/qag13/bofu3.js; rg -n 'kuduvo' pez/qag13/bofu15.js | lili", timeout:null}),
]);
return r.map(x=>typeof x==='string'?x:x.output).join("\n=========\n");
})(fake);
