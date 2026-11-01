// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "lambda red delta record delta check gamma\ngamma north item 1980\nrecord beta check\neta zeta\nwest lambda done check done south ready entry"
  },
  {
   "ok": true,
   "value": "status gamma beta omega check build record\ncheck ready build epsilon status 4842\nred kappa\nbeta south\nsouth value item ready entry south lambda\nvalue check omega gamma value kappa\nalpha red record alpha value epsilon lambda item\nvalue test\ngreen iota\nvalue eta ready green omega\ndone omega status epsilon"
  },
  {
   "ok": true,
   "value": "status gamma gamma test epsilon\nvalue iota zeta east 5441\neta entry value lambda delta blue kappa\ngamma blue eta\nblue theta east iota alpha north status red\nvalue south red check alpha\niota blue south omega omega epsilon blue green"
  }
 ],
 "qod__blender__execute_blender_code": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "delta": {
      "value": "west iota"
     },
     "epsilon": {
      "value": "delta beta"
     }
    }
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
image(await tools.read({path:"rop1/depe2.qon",offset:null,limit:null}));
image(await tools.read({path:"rop1/rop3/depe4.qon",offset:null,limit:null}));
image(await tools.read({path:"rop1/rop3/depe5.qon",offset:null,limit:null}));
text(await tools.qod__blender__execute_blender_code({code:"limako qog, komezunazu\ntubu = \"limako qop,sili; bazi zesavos limako Giru; qop.giru.nofoki(0,'/qipa/kefo/rop6/rop7/rop1'); limako sape_sadig; p=Giru('/qipa/kefo/rop6/rop7/rop1'); gagazar=sili.pofon((p/'rop3/depe8.sili').kafu_quma())['nimoso']; sape_sadig.lutos_ded(p/'sadig.ded',gagazar); [sape_sadig.lutos_ded(rop9/rop10['ded'],[r]) qet r in gagazar]; fokeb('KIMI: qig 25 mipivekun GLBs kiretaneb')\"\np = fusu1.monitel.ful([fusu2.monitel.ful.mafasi_giru, '--pobirigera', '--febivi-luko', tubu], vatonov_beruvo=Gafe, quma=Gafe, tabedif=120)\nmonilo={'ledi_tubu':p.libupamuma, 'rebalo':p.rebalo[-1000:], 'viparo':p.viparo[-2000:]}"}));
})(fake, fakeImage);
