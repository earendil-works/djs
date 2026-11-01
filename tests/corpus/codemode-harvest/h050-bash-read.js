// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export item build west\nconst south alpha gamma\nconst record ready"
  },
  {
   "ok": true,
   "value": "export theta item\nfunction eta\nexport alpha gamma zeta entry\nconst omega omega delta\nexport delta\n  if (value kappa done record\n  if (green west\n}north eta check\nconst beta"
  },
  {
   "ok": true,
   "value": "}alpha ready\nexport epsilon zeta lambda\nconst delta lambda\n  if (south done value north\nexport check\n  return ready blue status gamma"
  },
  {
   "ok": true,
   "value": "}delta blue gamma red\nexport entry omega record\nfunction gamma\n  return theta\nlet entry zeta north\n  return north value\n  return gamma red beta north\n  return build red\nlet delta\n}ready\n  return gamma east north east\nconst ready\n  if (alpha eta\n  if (status"
  },
  {
   "ok": true,
   "value": "let blue lambda\nconst eta delta east\n  return build zeta delta\nconst south lambda done"
  },
  {
   "ok": true,
   "value": "function alpha green green epsilon\n  if (blue\nconst alpha test north\nlet eta east\n  if (red\nfunction south lambda lambda theta\n  if (theta zeta test test"
  },
  {
   "ok": true,
   "value": "function alpha zeta status\nexport test\nfunction ready test value\nexport south\nfunction west east beta test\n  if (east item\n  if (south kappa epsilon\n  return epsilon zeta\nconst beta test omega\n  return omega gamma test\n  return iota\nfunction south\nexport green lambda zeta east\n  if (lambda blue omega"
  },
  {
   "ok": true,
   "value": "let red done red\n  if (theta value iota build\nfunction item west zeta iota\nfunction theta\n  return alpha\n  return record\nexport status zeta"
  },
  {
   "ok": true,
   "value": "}east\nlet green\n  if (ready\nfunction east green\n  return entry value omega kappa\n  if (entry zeta\n  if (east check\nlet theta\nexport beta west\nlet red east build\nexport entry lambda beta\nconst done gamma theta\nconst beta entry\n  return item check gamma"
  },
  {
   "ok": true,
   "value": "let delta beta\nlet blue record\n  if (delta blue\n  return eta gamma\n  if (delta delta build entry\nconst test west blue done\nlet alpha\n}epsilon\n  if (kappa ready check\n  return record red north ready\nfunction omega\n  if (entry alpha"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "item gamma north eta delta\nready value alpha epsilon 4981\nwest item iota ready check north entry entry\nbuild build check check check east south 6642"
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
for (const [path,offset,limit] of [
["dir1/file2.py",45,180],["dir1/file3.py",1,100],["src/dir4/file5.js",570,100],["src/dir4/file5.js",740,90],
["src/dir4/file5.js",840,110],["src/dir6/file7.js",270,40],["src/dir6/file8.js",130,67],
["src/dir9/file10.js",1,115],["src/dir9/file11.js",1,110],["src/dir12/file13.js",450,40]
]) text({path,content:await tools.read({path,offset,limit})});
text(await tools.bash({command:"rg -n 'litil|kotu|TAZU_KUKAFOMA' duzum/*.sovo.js | qata -80; rg -n 'nadavos|nafumo|vuzobo' qub/fiz14/{rufupas.js,fiz15/bogu5.js}; rg -n 'qaged|qodeGolag|litil' qub/fiz4/{nadavos,nalatubodo}.js; ls -l /fiz16/bogu17.qar/fiz18/fiz19/fiz20; rg -n 'zodozula gamelu|gamelu_qam|qat .*gamelu|qat .*dagezep|qat .*segi|qat .*moso|qat .*gegega' fiz1/bogu3.py"}));
})(fake);
