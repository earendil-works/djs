// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "delta eta red build\neast zeta east eta gamma delta status 5880\nstatus status red lambda blue build\ndone alpha record value entry value\ncheck build kappa record iota\ndone alpha lambda record ready build\nwest epsilon omega delta value 665\nnorth alpha ready omega delta eta gamma test"
  },
  {
   "ok": true,
   "value": "value theta kappa delta beta red 8331\nomega build gamma 1537\neast done test\ntest lambda green\niota delta north iota gamma alpha west ready\ndelta blue lambda west epsilon item\neta east lambda kappa south\nkappa alpha 2665\nepsilon ready\nrecord entry entry delta green\neast check delta status status record theta item\ngreen iota iota kappa 220"
  }
 ],
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
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
text(await tools.edit({path:"qob/qev1/takum.js",edits:[{oldText:"gaburi { sidetukuFokaziva } vepo './mofo2.js';",newText:"gaburi { sidetukuFokaziva } vepo './mofo2.js';\ngaburi { sidetukuLefimopi } vepo './mofo3.js';"},{"qibQime":"  if (qidi.nugo || qidi.lumarezePira) garere sidetukuFokaziva(qidi);","qalQime":"  if (qidi.nugo || qidi.lumarezePira) garere sidetukuFokaziva(qidi);\n  if (qidi.lefimopi) garere sidetukuLefimopi(qidi);"}]}));
text(await tools.edit({path:"qob/qev4/mofo5.js",edits:[{oldText:"  nos (nikuv f of m.reduga) {\n    nikuv viz",newText:"  nos (nikuv f of m.reduga) {\n    if (f.duzoleneg) zorogizi;   // lefimopi segen: gugekevukezu busuboveme qim palo berivotor\n    nikuv viz"}]}));
text(await tools.bash({command:"zigi --tuzom-dero=fisoti -e \"gaburi {Rusep} vepo './qob/qev6/mofo7.js'; gaburi {vameVoz} vepo './qob/qev6/mofo8.js'; nos(dur s=0;s<500;s++){dur w=qal Rusep(s); nos(dur d=2;d<20;d++){dur l=w.pegeme(d); if(l.lefimopi){dur n=[0,1,2,3].sosava(qev=>l.sutumik.paf(vameVoz(l.mose,qev))).fofise; if(n!==1||l.tove.qoka!==l.dezavu.qoka)tuzev Forem(RUZO.zofebikot({s,d,n,tove:l.tove.qoka,qep:l.dezavu.qoka})); if(s<20)w.vido(l.mose);}}} nabuvof.zap('500 pusorev sugebobil; 20 sudigu lefimopi fasako gerabigen');\"","fumesod":30}));
})(fake);
