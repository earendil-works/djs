// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "eta gamma value beta value gamma lambda 1031\nalpha omega entry build south build\nsouth theta\nred delta build item epsilon epsilon done 1026\nlambda ready theta theta north item done\nentry west west east\ntest red alpha epsilon east theta eta red 7796\nsouth ready ready delta blue done ready entry\neta east 6833\nbeta done blue entry test iota\nitem south kappa epsilon gamma"
  },
  {
   "ok": true,
   "value": "src/file4.ts:342: build eta\nsrc/file6.ts:357: zeta iota omega gamma\nsrc/file6.ts:206: green south test done\nsrc/file8.ts:116: iota value lambda beta check east\nsrc/file7.ts:207: entry theta red done theta\nsrc/file4.ts:38: entry status kappa lambda\nsrc/file8.ts:2: delta kappa\nsrc/file1.ts:146: west epsilon south west\nsrc/file9.ts:35: beta omega\nsrc/file4.ts:379: item omega theta value kappa delta"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "1. Iota eta status\nZeta lambda\n1. Eta item beta value theta done iota north\nIota build\n- Lambda beta alpha delta build\nZeta green red kappa item\n## Check build west\n- Done red status item iota"
  },
  {
   "ok": true,
   "value": "}value\nconst value check\nlet kappa south item\nconst eta lambda gamma iota\n  if (omega record\nfunction blue\nlet ready\n}omega status omega\nfunction delta zeta status gamma\n}test eta east record"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
text(await tools.bash({command:"pobi --kulom-gamu=ditebi <<'JS'\nmidafe { Sivom } qole './qet/zun1/mabo2.js';\nmidafe { lezegude } qole './qet/zun3/nepos.js';\nmidafe { PIPOGIZE, fibeDesefo } qole './qet/zun1/mabo4.js';\nkuv (qad roko=0;roko<10;roko++) {\n qad befu=tepi;\n kuv(kupem [zusir,dimafudi] of [[13,['dodatan','gizebop','zofetutono']],[15,['dozoza','qore','kurokazup']]]) {\n  kupem w=tek Sivom(roko), dez=w.valani(zusir).fibeZenis.vozu().qole().pofid; w.zikasib(dez); kupem vidi=w.vidi(dez);\n  kuv(kupem pipogize of PIPOGIZE) kuv(kupem defoviz of dimafudi) { kupem m=lezegude({...vidi,rasaseVefez:[{...fibeDesefo(roko,dez,vidi.lofav,pipogize),defoviz}]}); if(m.rasaseVefez.ninazo!==1)befu=diran; }\n }\n ditobug.del(roko,befu); if(befu)likam;\n}\nJS","zivenem":120}));
text(await tools.bash({command:"rg -n 'libeQilosZigiv' qet/zun5; ls dedip; rg -n 'guzoseme|zusir|nomaz|sosiga' REROZU.md qilo/mabo6.md mizom/zun7/mabo8.js mizom/zun7/mabo9.js","zivenem":null}));
text(await tools.read({path:'qilo/mabo6.md',offset:null,limit:null}));
text(await tools.read({path:'mizom/zun7/mabo8.js',offset:null,limit:null}));
})(fake);
