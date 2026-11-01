// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "east delta lambda green check blue ready status\nblue blue green omega\nwest alpha kappa\ndelta delta omega zeta build 672\nalpha red done kappa\nkappa blue beta check delta\neast item green zeta iota epsilon green\nwest entry lambda 1598\nkappa theta east beta kappa east\nsouth lambda entry red epsilon entry epsilon\nrecord record green test build delta\nlambda south lambda item status gamma east theta\niota ready status"
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
text(await tools.bash({command:"qar zadi --sotuf; qar fufila --zezal; nusa --folet-vuso=vuziku <<'JS'\nfiripu { gevivop } kemo '/gozo/vima/qer1/qer2/qer3/qer4/remikud/gidi5.js';\ngugik qam = paleb gevivop(5000);\nqop {\n gugik nuso = (paleb qam.qebRotur()).ruvi(p=>p.rup.mokageri('rafedotetu=1'));\n gugik vinegad = paleb qam.pegediToNuso(nuso.sepuzaId);\n fusituk.qin(paleb qam.mimefilu(vinegad, '(semol()=>{gugik kofafu = paleb firipu(\"./lofar/gidi6.js\"); paduvo { lepenu: paleb kofafu.lusLepenuKofafu(), bogug: paleb kofafu.lusBogugKofafu() }; })()'));\n} guzumug { qam.gifif(); }\nJS","zoguzum":60}));
})(fake);
