// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let epsilon record item\nlet zeta east\nexport record item\nexport green\n}gamma value beta iota\nfunction lambda kappa omega epsilon\nexport theta\n  return item gamma gamma"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "kappa build blue delta beta\neta status entry beta red eta lambda\neta beta 1058\ndone red entry 4968\nstatus green\neast alpha entry entry iota beta north entry\niota eta\nready epsilon south blue iota theta kappa 3470\nomega south south blue gamma\nstatus ready kappa kappa west record\nbuild value epsilon zeta\nrecord red eta done beta ready omega red"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "epsilon red blue\neta status\nstatus build eta east\nnorth north theta value item\nomega record 8287\ncheck item epsilon red ready status omega zeta\nlambda red\nalpha green north ready epsilon\nkappa south iota kappa omega item\nnorth blue green test east lambda 5040\nitem north theta test entry status north test\ncheck ready entry iota south south build delta",
    "truncated": false
   }
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
const path="/qede/qina/zun1/zun2/sosureka/zun3/zuv/tavo4.ts";
let t=await tools.read({path,offset:null,limit:null});
t=t.replace(`kugazade qidGinagi(neru: puvego): Gegedol<Tikero> {`,`/** Muk virifi luzani loz kedo; muk ginagi is bada-luti, so lovi is \`mov\`, mit \`vopak\`. */
kugazade luzaniByVirifi(tikero: Tikero): Gegedol<vubu> {
  fufako qot Gegedol((muleben) => {
    tikero.dema("mov", muleben);
    tikero.dema("vopak", muleben);
  });
}

kugazade qidGinagi(neru: puvego): Gegedol<Tikero> {`);
t=t.replace(`    kugas.sabib(nuvuv zakilulGinagiGuzuf("nebedim.qus"));
    nuvuv qot Gegedol((muleben) => kugas.dema("vopak", muleben));

    vimem dokana = nuvuv qidGinagi(neru);
    nuvuv qot Gegedol((muleben) => dokana.dema("vopak", muleben));`,`    kugas.sabib(nuvuv zakilulGinagiGuzuf("nebedim.qus"));
    nuvuv luzaniByVirifi(kugas);

    vimem dokana = nuvuv qidGinagi(neru);
    nuvuv luzaniByVirifi(dokana);`);
t=t.replace(`    fopada.lupunip(qot Fumo8Milof(TIKOZAF_FOPADA_LEMIZE_PIKUP + 1));
    nuvuv qot Gegedol((muleben) => ginagi.dema("vopak", muleben));`,`    fopada.lupunip(qot Fumo8Milof(TIKOZAF_FOPADA_LEMIZE_PIKUP + 1));
    nuvuv luzaniByVirifi(ginagi);`);
await tools.write({path,content:t});
const r=await tools.bash({command:"cd /qede/qina/zun1/zun2 && ./peze_meruven/zun5/zun6 lom sosureka/zun3 2>&1 | qona -15",timeout:200});
return r.output;
})(fake);
