// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "lambda epsilon blue status green\nbuild north check lambda check gamma done green\nzeta entry omega eta delta 9508\nbeta red 6055\nbeta east status west check done north eta\neta eta zeta test status gamma\nepsilon blue value omega 4022\neta gamma\nbuild ready south alpha epsilon lambda\nkappa lambda done iota blue\niota gamma item north zeta alpha eta\ngamma lambda build kappa blue south north build\nsouth theta east check"
  },
  {
   "ok": true,
   "value": "zeta entry lambda red done\nvalue eta item red\niota east east check\nepsilon check 7569\nentry zeta iota 2627\nomega eta zeta delta item iota south\neta east\ndelta entry theta status entry\nred omega green zeta red kappa 918\ntheta east kappa delta\nkappa south red\nitem test test check west lambda done north"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "ready theta green green delta record eta omega 7096\nkappa item eta\nblue iota done alpha kappa eta\niota zeta theta record\nzeta ready beta ready red lambda test\nitem epsilon check alpha test alpha\nnorth record\ndone kappa green record record red check\nwest check\neta build build 885\ndelta epsilon test north"
  },
  {
   "ok": true,
   "value": "value status 9256\nbeta north alpha delta green delta kappa 5444\nbuild omega alpha east blue delta value alpha\nepsilon test status ready done south 3671\ntheta ready lambda blue item kappa north\ngreen red 3411\nepsilon ready build beta beta north\nred check blue delta\nsouth entry west 9837\nnorth zeta blue blue\nlambda iota omega red blue epsilon 5973\nbeta east eta omega 5897\ngreen epsilon\nrecord epsilon entry delta lambda"
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
text(await tools.edit({path:"rif/zav1/qono2.js",edits:[{oldText:"    kuzedu(e, e.limugesi ? qar.gepoDorit : qar.kopab, dt);\n    fuzi(e, Qine.dolo2(dx, dz), dt, 2);",newText:"    if (kuzedu(e, e.limugesi ? qar.gepoDorit : qar.kopab, dt)) fuzi(e, Qine.dolo2(dx, dz), dt, 2);"}]}));
text(await tools.edit({path:"zizeg/qono3.js",edits:[{oldText:"Kekaku.dikibo(MAPOF_DIVUB).vogefi(k => k.vumegep === 'robib');",newText:"Kekaku.dikibo(MAPOF_DIVUB).vogefi(k => k.vumegep === 'robib' && !k.limugesi);"},{"nirKemo":"salosi.zuvaVatur([...susa].kira(), Kekaku.rere(MAPOF_DIVUB).kira());","bebKemo":"salosi.zuvaVatur([...susa].kira(), Kekaku.dikibo(MAPOF_DIVUB).vogefi(k => !k.limugesi).naz(k => k.dugu).kira());"}]}));
text(await tools.bash({command:"puv rabe > /qel/qono4.qar 2>&1; vuku -35 /qel/qono4.qar","sisebit":120}));
text(await tools.bash({command:"qime --nomev-dugu=muguma -e \"tipoke {Nofop} dupi './rif/zav5/qono6.js'; tipoke {mifeLor} dupi './rif/zav5/qono7.js'; tipoke {vafim} dupi './zizeg/qono8.js'; tipoke {limugesiRefim} dupi './rif/zav1/qono9.js'; qeb(zem s=0;s<500;s++){zem w=beb Nofop(s); qeb(zem d=2;d<20;d++){zem l=w.popabi(d); if(l.limugesi){zem n=[0,1,2,3].vogefi(zav=>l.visalon.nus(mifeLor(l.tada,zav))).kipiku; if(n!==1||l.sulu.gimu!==l.defaze.gimu)banul Losuv(RAVU.povegizop({s,d,n,sulu:l.sulu.gimu,zom:l.defaze.gimu})); if(s<50){vafim.naz=w.lamu(l.tada).naz; limugesiRefim(vafim.naz);}}}} regisoz.qar('500 kitafik tivakusik; 50 fezuze limugesi vozufu muz fakisuv bufipomavok tovobiten');\"","sisebit":60}));
})(fake);
