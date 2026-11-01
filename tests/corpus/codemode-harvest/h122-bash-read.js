// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "## East beta blue iota build\nTheta kappa lambda omega record\n## Red done done\n## North entry\n- Zeta record beta north west\n- Test iota epsilon done test entry blue alpha\n# Beta lambda iota\n- Build kappa eta test alpha epsilon\n# East entry east value item\nRed green record iota blue gamma record\nStatus epsilon alpha"
  },
  {
   "ok": true,
   "value": "West east value record\n- Blue north\n1. Eta check item gamma check\n## Delta eta entry lambda value lambda build\n# East record"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file9.ts:222: west item south ready\nsrc/file3.ts:306: status east\nsrc/file7.ts:310: eta theta check"
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
const r = await Promise.all([tools.read({path:"NUMAT.md",offset:null,limit:null}), tools.read({path:"qon1/FOZAMU.md",offset:null,limit:null}), tools.bash({command:"qapo -n '^#\\|^- \\*\\*Q\\|^### Q\\|^## ' REFOZOZIR.md | fuku -150",timeout:null})]);
return r.map(x=>typeof x==='string'?x:x.output).join("\n=====\n");
})(fake);
