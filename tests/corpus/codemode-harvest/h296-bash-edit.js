// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "entry build\niota omega build green north\niota north omega 3891\ngreen kappa\nvalue delta build\nlambda iota beta north done test\nbeta delta south west entry north 4328\nalpha west south 3864"
  },
  {
   "ok": true,
   "value": "delta beta done record theta theta 1825\ngreen omega check check green\nsouth red eta zeta status test item iota\ngreen green test record 7867\nbeta ready 876"
  },
  {
   "ok": true,
   "value": "west red red status status south south\ndone done delta\niota alpha east omega\ndone blue check south entry kappa east 4417\ncheck beta alpha"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "status item alpha omega beta beta\ntheta alpha green check value\ngamma alpha alpha north\neta kappa record kappa build gamma omega beta 3291\nentry build test 5771\niota build kappa zeta alpha red\nbuild delta west blue check red 2748\neast entry omega zeta\nred beta red ready red value ready 8595\nitem east beta alpha alpha check 7018\nbeta red eta zeta west iota omega 5241\nsouth value zeta record north build eta delta\neast blue delta green theta check"
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
await Promise.allSettled([
tools.edit({path:'fab1/dez/fab2/feka3.js',edits:[{oldText:"mivufovoVudad(pavi, qema, [[5.6, 8.2]",newText:"mivufovoVudad(pavi, qema, [[5.6, 7.2]"}]}),
tools.edit({path:'fab1/zuna/feka4.md',edits:[{oldText:"on fivipevesa gizoziz, faditupisel migaf or qit `?vizal=` takek kifapoke. Zotitukur, bugi, rinul nad vudad qit nulesase sezupi\nmazokal sagadogumi or piluzunigi zilagisisuze.",newText:"on fivipevesa gizoziz, faditupisel migaf or qit `?vizal=` takek kifapoke. Zotitukur,\nbugi, rinul nad vudad qit nulesase sezupi mazokal sagadogumi or piluzunigi\nzilagisisuze."}]}),
tools.edit({path:'fab1/gufop/feka5.js',edits:[
{oldText:"    pagilPeritenKolelu(m);\n    zozebu.rudig(m.qula.zidinekota.gagofo",newText:"    pagilPeritenKolelu(m); pagilNibiloVudad(m);\n    zozebu.rudig(m.qula.zidinekota.gagofo"},
{oldText:"    pagilPeritenKolelu(m);\n    zozebu.kenaRudig(fosumSoneg(m, GORUDA), []);",newText:"    pagilPeritenKolelu(m); pagilNibiloVudad(m);\n    zozebu.kenaRudig(fosumSoneg(m, GORUDA), []);"},
{oldText:"suroguzu pagilPeritenKolelu(m) {",newText:"suroguzu pagilNibiloVudad(m) {\n  kiviv pavi = m.qula || m.posedareLusa;\n  qip (kiviv qema of pavi.vudad) qip (kiviv [i, b] of qema.gafupa.dibeg(1).kapores()) {\n    kiviv a = qema.gafupa[i];\n    qip (qak t = 0; t <= 1; t += .1) {\n      kiviv x = a[0] + (b[0] - a[0]) * t, z = a[1] + (b[1] - a[1]) * t;\n      zozebu.ok(!m.dotitepiz.gagofo(b => b.zuzup !== 'fesivameg').sapi(b => turozoraXZ(b, x, z, FURI_R + .03)), 'nibilo qema qofa kifun ronode, qom dino gasu puvazisap');\n      kiviv c = Kofu.nis(N - 1, Kofu.qed(0, Kofu.bonat(z))) * N + Kofu.nis(N - 1, Kofu.qed(0, Kofu.bonat(x)));\n      zozebu.rudig(m.qipe[c][0].qugus, 0);\n    }\n  }\n}\n\nsuroguzu pagilPeritenKolelu(m) {"}
]})
].map(async p=>text(await p)));
text(await tools.bash({command:'cd mapife && sep vufo > /fuk/feka6.qat 2>&1; zegu -10 /fuk/feka6.qat',timeout:120}));
})(fake);
