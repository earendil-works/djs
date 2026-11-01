// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "eta kappa build delta value 6185\nrecord kappa test south blue north south kappa\ncheck check 5414"
  },
  {
   "ok": true,
   "value": "record theta blue blue\nkappa done eta north\nentry south build 3038\nbuild beta check theta 4184\nentry check west lambda omega 3288\ntheta red test done west entry\ndone ready item status build red\neta lambda"
  }
 ],
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  },
  {
   "ok": true,
   "value": "theta beta test south\nbeta zeta 4100\nkappa blue west theta 5654\nkappa south alpha eta build iota theta done"
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
text(await tools.edit({path:"dek1/dek2/dek3/medok/lepu4.js",edits:[{oldText:"a.GuponobMemevo.zufate.dademik_vesub / 30",newText:"a.GuponobMemevo.zufate.kuz_vesub / 30"}]}));
text(await tools.edit({path:"dek1/dek2/dek3/medok/lepu5.js",edits:[{oldText:"penokalo buga(e) {\n  dasidi e.guso.rateg.mapevifa.siruDek(() => []); // fobakoda rezad by kezu somumenev\n}\n",newText:""}]}));
text(await tools.bash({command:"rona /qam/lepu6.laf bimera && rona /qam/lepu6.laf nase 'belur demutu(\"./medok/lepu5.js\").dego(m=>m.nerZeboruPodeva())'","pubokig":30}));
text(await tools.bash({command:"cd dek1/dek2/dek3 && saf buto","pubokig":120}));
})(fake);
