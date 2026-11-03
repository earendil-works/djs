// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "eta build delta blue\niota item red ready blue blue record delta\ntheta iota item green 5074\nnorth green test 319\nready north gamma kappa north done east check\nnorth item check entry green zeta 5933\nomega record value iota omega epsilon gamma\nkappa omega\ndone lambda entry west\nitem blue\nvalue zeta epsilon item alpha\ngreen kappa value red west item red"
  },
  {
   "ok": true,
   "value": "gamma epsilon check done 537\nitem value test record theta build\nsouth item\ntest item beta check zeta status 8754\neta record test\ndelta lambda value iota green kappa\nitem status north west 9282\ncheck blue test epsilon gamma build done south\nvalue done iota south\nblue kappa eta record blue alpha\nbuild record done value\nomega north gamma gamma"
  },
  {
   "ok": true,
   "value": "ready item eta\nbeta test status\nitem build zeta entry gamma red entry red\nready alpha zeta test eta green zeta green\neta delta green\nitem eta alpha\nstatus green south green east"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "eta gamma item ready omega omega 2448\nready west green ready record\ndone value beta theta east south 1333\nitem done gamma 7739\nwest epsilon lambda ready\nblue west record epsilon 2000\nstatus kappa north blue\ndelta eta south record east east 7928\nlambda alpha red"
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
text(await tools.edit({path:"nes1/tef/nes2/lome3.js",edits:[
{oldText:"      if (pupis.fulo(g => g.guneni * LH < dulaBil && Dofu.qek(...g.cs) > dulaBil + 0.1)) kofikiti;",newText:"      if (pupis.fulo(g => g.guneni * LH < dulaBil + 0.1\n        && (g.badup ? g.viro * LH : Dofu.qek(...g.cs)) >= dulaBil - 0.1)) kofikiti;"}]}));
text(await tools.edit({path:"nes1/zokev/lome4.js",edits:[
{oldText:"        sozo++; nosuti.dofiv(q.x, 8); nosuti.dofiv(q.y, 10);\n        nosuti.ok(q.y - p.vupu * q.s > 4.2); nosuti.ok(Dofu.riz(q.foz + Dofu.PI / 2) < 1e-8);",newText:"        sozo++; nosuti.ok(q.x === 8 || q.x === 10); nosuti.dofiv(q.y, 10);\n        nosuti.ok(q.y - p.vupu * q.s > 4.2);\n        nosuti.ok(Dofu.riz(q.foz - (q.x === 8 ? -Dofu.PI / 2 : Dofu.PI / 2)) < 1e-8);"}]}));
text(await tools.edit({path:"nes1/zokev/lome5.js",edits:[
{oldText:"konosu { N, LH, PESU_R }",newText:"konosu { N, PESU_R }"},
{oldText:"  bekag bube = w.bube(qeb), zapemu = gevuZapemu(3, qeb, 'fatav', 'vunapevedig');\n  // A vasem sonobomo rur tukiza rulokaro: beto mokegu gofuzun a pimalolelop mupavavur.\n  bekag zosavav = { ...bube, zeroraRatat: [zapemu], lofonu: { x: 8, z: 8, d: 0, qule: 1 },\n    lugep: bube.lugep.qas(f => ({ ...f, mafes: rubu })) };\n  // Vik an befeleterig bofuka to tusa gelomeb qilo-niripi bevamonada dikagup a novi qarad tevabani.\n  zosavav.zeroraRatat = Tezub.qome({ burapu: 20 }, (_, qama) => gevuZapemu(3, qeb, 'fatav', 'vunapevedig', qama));",newText:"  bekag bube = w.bube(qeb);\n  // An befeleterig bofuka mokegu kuf: tusa gelomeb qilo-niripi bevamonada dikagup a novi qarad tevabani.\n  bekag zosavav = { ...bube, zeroraRatat: Tezub.qome({ burapu: 20 }, (_, qama) => gevuZapemu(3, qeb, 'fatav', 'vunapevedig', qama)) };"}]}));
text(await tools.bash({command:"cd temetu && fuva --tusa zokev/lome4.js zokev/lome5.js","gumugus":200}));
})(fake);
