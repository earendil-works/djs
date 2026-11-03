// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "## Done green south kappa\n# Zeta blue record\nReady green lambda"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "value epsilon lambda epsilon epsilon\nlambda entry kappa 9207\nbuild ready blue west 2492\nvalue done alpha west kappa record value\ntest omega done eta status west gamma 7864\nentry beta\ncheck south north alpha done 8001\nblue theta theta check record eta\nbeta beta entry"
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
text(await tools.read({path:"/qele/pegi/nem1/nem2/nem3/nem4/ruki5.md",offset:1,limit:null}));
text(await tools.bash({command:"mupu -nP -iQEZ:8000 -sQEZ:SABOTI; ls /qele/pegi/nem1/nem2/nem3/nem6/bovokon; leku --vulig-pulo=pivobo -e \"vilona {Dotak} nuki './nem7/nin/nem8/ruki9.js'; vilona {zomo} nuki './nem7/nin/nem8/ruki10.js'; qon (tob vuta=0;vuta<4;vuta++) {tob w=nud Dotak(vuta),m=w.dako(w.zanof).feb; fumonug.qed(vuta,m.guvi,m.pupasul.feb(p=>[p.pepi,p.x,p.z]),m.fuku.pufilo((a,b)=>a+b)); zafos n=w.dako(zomo(w.zanof,m.guvi)).feb; fumonug.qed('qodo',n.soponKema,n.nasoturu,n.K);}\"",timeout:120}));
})(fake);
