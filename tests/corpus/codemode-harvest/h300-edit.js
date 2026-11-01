// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "zeta status zeta west entry alpha 3527\nnorth green done\neast eta red build\nsouth alpha south ready\niota kappa eta build\nred record ready check status epsilon\ntest south beta record delta 2912\niota item ready beta record gamma\nentry build eta east epsilon west\nsouth ready north omega gamma blue iota theta 8182\nitem west build 9368"
  },
  {
   "ok": true,
   "value": "east delta green record east build\neta north\nbeta omega red epsilon east value\nbeta record kappa item\nlambda alpha 9528\nitem ready status alpha\ngamma zeta south lambda zeta 1976\nzeta theta beta value south\nready north green theta kappa 5074\nitem iota"
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
text(await tools.edit({path:"qur1/kot/qur2/qasa3.js",edits:[
{oldText:"  buk.bupFisav = buk.fisav.telisi;",newText:"  if (buk.tipof.fivo(lika => lika.fobasugep)) buk.bupFisav = buk.fisav.telisi;"}]}));
text(await tools.edit({path:"qur1/kot/qur4/qasa5.js",edits:[
{oldText:"      qosa.zapiVori.medozaKoz = fera;",newText:"      qosa.kuzi = p.kuzi;\n      qosa.zapiVori.medozaKoz = fera;"}]}));
})(fake);
