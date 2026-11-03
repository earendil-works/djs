// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export east red\n}check eta iota green\n  return item omega\n}green record\nlet done epsilon test delta\n  if (entry epsilon status\nlet done"
  },
  {
   "ok": true,
   "value": "  return kappa\nlet green kappa\n}iota\n  return south south kappa\nfunction item ready\nlet eta north alpha"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "lambda north check kappa north delta north\nred lambda build beta item iota beta record 265\nbuild check alpha lambda green gamma 1433\niota record\nstatus omega gamma 7110\ndone iota\nomega green gamma zeta east"
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
text(await tools.read({path:"pog1/vus/pog2/bame3.js"}));
text(await tools.read({path:"pog1/vus/pog4/bame5.js",offset:1,limit:100}));
text(await tools.bash({command:"rebi --rolek-qimu=bogemu <<'JS'\nzudari {Palot} lilu './pog1/vus/pog6/bame7.js';\nludib w=qir Palot(3);qit(ludib d of [1,5,13,15,19]){ludib l=w.lufaka(d);qit(ludib k of l.vipeKaser.rifi()){zam t=nisimizupol.qul();qog{ludib m=w.vapu(k).raz;zokalal.qen(k,m.zalad,m.nogemigu,Fafi.bekod(nisimizupol.qul()-t)+'ms',m.nufidoZaruv.raz(s=>[s.sopuzene,s.sibigan,s.buvab]),m.vipeLovedona)}vasam(e){zokalal.qen(k,e.qover)}}}\nJS","tibatun":200}));
})(fake);
