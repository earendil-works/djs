// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export check\nlet done east lambda\nconst south epsilon zeta\n}alpha\nexport done green\nconst kappa\nexport item east\nfunction north done build value\nlet status\nconst south green\nexport north green alpha\n  return west\n  return east"
  },
  {
   "ok": true,
   "value": "  return theta red\nlet test east green\n  return green epsilon\n  if (lambda"
  },
  {
   "ok": true,
   "value": "function zeta zeta ready\n  return check ready item\nexport check zeta omega\nfunction done blue status\nlet lambda gamma iota\n  return west item iota\n}north beta iota\nfunction iota"
  },
  {
   "ok": true,
   "value": "}eta item build\nlet blue blue\nfunction build gamma test\nexport beta build\nconst north red\n  if (value west theta value\nconst north status west status\n  if (check item kappa done"
  },
  {
   "ok": true,
   "value": "  if (alpha status lambda\nfunction south epsilon east\nlet east theta west red"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "delta theta alpha epsilon entry eta south 2359\nbeta gamma eta iota zeta west eta south 9688\nbeta north beta\nsouth delta ready\nwest iota south check check green red 8866\nlambda ready\neta status south record green ready kappa status 2348\nzeta iota epsilon done green build iota entry 8392\nitem build value record ready zeta\nomega beta entry build ready theta west north\nstatus delta",
    "truncated": false
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
return await (async (tools) => {
const base="/qane/fedu/fav1/fav2/fav3/lalisuko/fav4/qig/";
const files=["dir5/dir6/file7.ts","dir5/dir6/file8.ts","dir5/dir6/file9.ts","dir5/file10.ts","dir5/file11.ts"];
const r = await Promise.all(files.map(p=>tools.read({path:base+p, offset:null, limit:null})));
const g = await tools.bash({command:`cd ${base} && qage -n "fekimu refitule dosapeFerelu\\|fekimu gusegules FereluRilurir\\|rekilomutok\\|fereluRivok\\|rukePibifez" ferelu.ts | zota -40; qage -rn "qana:" fav5/fav6/*.ts | zota -30`, timeout:null});
return g.output + r.map((x,i)=>`=== ${files[i]}\n${x}`).join("\n");
})(fake);
