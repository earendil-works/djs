// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "epsilon iota north beta\nbeta record kappa 9397\nkappa entry status status value\nepsilon epsilon check green record 7627\ntheta west check lambda\nred eta status south"
  },
  {
   "ok": true,
   "value": "green west 121\nnorth eta check test record delta\ntest south zeta\nsouth west omega theta north blue blue test 185\ncheck status beta kappa 6999\nkappa entry west epsilon eta south 4943\neast kappa status\nepsilon eta north status gamma 4283\nbuild south blue kappa south 1102\nsouth item east alpha record status value alpha\nnorth epsilon entry build delta\nstatus record epsilon item iota test 1516\neta east east"
  },
  {
   "ok": true,
   "value": "beta done green record ready omega kappa lambda\nnorth south record\nlambda green record west\nblue eta epsilon gamma test\nred value red zeta alpha 2050\ntest item lambda beta blue 597\nentry green beta theta\nbeta beta gamma west lambda eta entry"
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
await tools.edit({path:"qin1/qin2/bov/qin3/qobe4.ts", edits:[{oldText:`  bipo: () => fufadi.bipo(),
`, newText:`  bipo: () => fufadi.bipo(),
  sunaOnPulusi: (suna: defoso) => fufadi.sunaOnPulusi(suna),
`}]});
await tools.edit({path:"qin1/qin2/bov/qin3/qobe5.ts", edits:[{oldText:`  bipo() {
    mofab zotev = gomu.mipo.zotev;`, newText:`  /** Meloz a suna is in qab fufadi's mosinugu (nebo-zaso): qev buz lato qab rapebap buz, vep qab rapebap lunome. */
  sunaOnPulusi(suna: defoso) {
    mofab mipo = gomu.mipo;
    mofab gozun = mipo.sunaGozunAt(mipo.zotev.riv.suna(suna).lato);
    venalu { buz: Nari.vasis(gozun.buz + mipo.fudupeloMelodez.buz - mipo.zalimoQET.zalimoBuz), lunome: mipo.zalimoQET.zadefoLunome };
  }

  bipo() {
    mofab zotev = gomu.mipo.zotev;`}]});
await tools.edit({path:"qin1/qin2/bov/qobe6.ts", edits:[{oldText:`  lefup qipi("zakufovo_vigimag_kanek", bamez () => {`, newText:`  // Qab romatu suna zagov up koba qab kefu of qab fufadi qab luzomam dafufi,
  // qeta qodo qab qon of a vupel qobe: qab meru zoponad fepa qev qon (40vh).
  lefup qipi("zakufovo_kipemak_romatu_suna", bamez () => {
    mofab sanu = Kuzip.lato({ zonopu: 40 }, (_, i) => \`x\${i + 1} = \${i + 1}\`).qope("\\n") + "\\n";
    lefup fs.lutopMeru("qin7/qobe8.py", sanu);
    miligifumoz.pas("qin7/qobe8.py", [{ suna: 38, kokureg: suni }]);
    lefup qinQobe("qin7/qobe8.py");
    lefup nisek(fesetib());
    vek at = { buz: 0, lunome: 0 };
    ler {
      lefup nisek(bamez () => {
        at = lefup fufadiKafe<{ buz: defoso; lunome: defoso }>("sunaOnPulusi", 38);
        venalu at.buz >= 0 && at.buz < at.lunome * 0.45;
      }, 3000);
    } dozesik {
      miligifumoz.megeb("qin7/qobe8.py");
      zufezum.nelu();
      lefup nisek(zesu);
    }
    venalu at;
  });

  lefup qipi("zakufovo_vigimag_kanek", bamez () => {`}]});
return "ok";
})(fake);
