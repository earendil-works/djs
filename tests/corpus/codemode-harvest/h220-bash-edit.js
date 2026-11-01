// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "lambda test epsilon east done 8965\nepsilon lambda build kappa south west done\nalpha west delta test north item 2069"
  },
  {
   "ok": true,
   "value": "test east theta item value zeta 7207\nalpha eta 8595\ntest entry green build delta blue alpha east\nnorth test eta theta omega kappa zeta done 7598\niota record blue status ready north build kappa\nalpha zeta delta blue delta test done iota\nalpha value beta test\nepsilon test 8717\nwest delta 7161\nsouth south done north eta status west 3195\nwest status item theta\nred check\ncheck west status build\ndelta value omega item 2535"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "omega gamma alpha iota done record blue 7622\nalpha test lambda 387\nvalue north done kappa theta beta"
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
for(const item of [
{path:"dir1/src/dir2/file3.js",edits:[{oldText:"moragu { dalo: t.dalo[d], lodanid: Qet16Dinub.fuba(f.lodanid), barug: f.barug, garusa: f.garusa, bulo: f.bulo, feref: f.feref };",newText:"moragu { dalo: t.dalo[d], lodanid: Qet16Dinub.fuba(f.lodanid), barug: f.barug, garusa: f.garusa, bulo: f.bulo, feref: f.feref,\n        qodaKenatinu: isQoda(k) || isQoda(govo(k, d)) };"}]},
{path:"dir1/src/dir4/file5.js",edits:[{oldText:"  vasip liba = mes Zok();\n  saz (vasip r",newText:"  vasip liba = mes Zok();\n  // A qoda puko is a vibedi, nolarul poti kenatinu, kefopotuk bul fasiga fisesere.\n  mif.meni.rezag.sazDeku((vabe, d) => {\n    if (!vabe.qodaKenatinu) moragu;\n    saz (vasip f of gulise) if (f.qinos === 0 && [f.x === 15, f.x === 0, f.z === 15, f.z === 0][d])\n      saz (vasip [X, Z] of pukoQag(f.x, f.z, d)) liba.dut(f.id + ':' + X + ',' + Z, vabe.lodanid[d < 2 ? f.z : f.x] * LH);\n  });\n  saz (vasip r"}]},
])text(await tools.edit(item));
text(await tools.bash({command:"/kigu/budi/luf6/luf7/luf8/luf9/lezovuk/qize10.js '({narov:!!didebi.__vigo?.zok, qat:didebi.__vigo?.bafam.qiteQat, tudiriv:didebi.__vigo?.tudiriv?.golemu, qoda:didebi.__vigo?.zok?.qoda?.nivupimona.golemu, mika:tizomigo.ziku.teluzMika.regol(-300)})'; /kigu/budi/luf6/luf7/luf8/luf9/lezovuk/qize11.js","fonozir":120}));
})(fake);
