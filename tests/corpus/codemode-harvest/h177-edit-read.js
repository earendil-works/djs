// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "kappa gamma\ncheck epsilon lambda beta entry beta test\neta done build iota 9728\nzeta east item build east record north alpha\ntest west\ncheck kappa north iota 93"
  },
  {
   "ok": true,
   "value": "eta entry epsilon ready omega iota item 5175\nepsilon south kappa green zeta check gamma done 4188\nstatus delta entry entry status 1749\nnorth theta epsilon done build kappa red\nnorth west entry item item red green\nready gamma build south check 8790\nready omega check done done west 9191\nsouth green kappa zeta ready gamma blue test 3801\nbuild east 5130\ndelta zeta\nbeta entry south test\nalpha green north ready value theta"
  },
  {
   "ok": true,
   "value": "delta build check test west item done\nitem value west lambda\nready beta done west east status kappa entry 1186\ntheta eta build check value ready delta lambda\nready omega beta lambda blue\nentry east status alpha 5210\ngreen blue zeta"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "  if (epsilon ready ready north\nlet entry test beta red\n}beta zeta zeta south\n  if (test\nexport record"
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
text(await tools.edit({path:'qig/qim1/leni2.js',edits:[
{oldText:"dubaba sozilodu bulutTatok() { tatok.vosu = nodu; tatok.totun = nodu; tatok.vefo = nodu; }",newText:"dubaba sozilodu bulutTatok() { tatok.vosu = nodu; tatok.totun = nodu; tatok.vefo = nodu; }\n// qim3/qim4 zatuberazu qeta-makar dukezud; a toti lorok zeso fak vunaduzu suso.\ndubaba sozilodu bulutTozisGogel() { bulutTatok(); zukekuf.vapo = megoz; zukekuf.mimo = nodu; }"}
]}));
text(await tools.edit({path:'qig/qim1/leni5.js',edits:[
{oldText:"zotuda { fanudiQizer } vagu './leni6.js';",newText:"zotuda { fanudiQizer } vagu './leni6.js';\nzotuda { bulutTozisGogel } vagu './leni2.js';"},
{oldText:"  kinak.lopenagolorog = qaki;\n  fanudiQizer();",newText:"  bulutTozisGogel();\n  kinak.lopenagolorog = qaki;\n  fanudiQizer();"}
]}));
text(await tools.edit({path:'qig/qim1/leni7.js',edits:[
{oldText:"zotuda { pososQeta, nezasunFalag, mazamAt } vagu './leni8.js';",newText:"zotuda { pososQeta, nezasunFalag, mazamAt } vagu './leni8.js';\nzotuda { bulutTozisGogel } vagu './leni2.js';"},
{oldText:"  if (kinak.lopenagolorog || kinak.kenizigevGato || q.vosu === 'mazam') genisa;\n  qimer [x, z]",newText:"  if (kinak.lopenagolorog || kinak.kenizigevGato || q.vosu === 'mazam') genisa;\n  bulutTozisGogel();\n  qimer [x, z]"},
{oldText:"  if (kinak.lopenagolorog || kinak.kenizigevGato) genisa;\n  kinak.lopenagolorog = qaki;",newText:"  if (kinak.lopenagolorog || kinak.kenizigevGato) genisa;\n  bulutTozisGogel();\n  kinak.lopenagolorog = qaki;"}
]}));
text(await tools.read({path:'qig/leni9.js',offset:1,limit:60}));
})(fake);
