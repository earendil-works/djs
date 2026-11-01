// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export iota item record eta\n  if (item\n  return south\n  if (lambda item red beta\n}ready\n  return value beta eta west"
  },
  {
   "ok": true,
   "value": "export lambda west ready kappa\n  if (green red east theta\n  if (value\nlet status iota check\nexport theta green check west\n}zeta test check status\n  return blue\n}alpha green red beta\nexport beta"
  },
  {
   "ok": true,
   "value": "  if (item delta beta\n  return epsilon lambda alpha north\nexport iota epsilon east north\nconst gamma\nfunction epsilon done\nfunction south east blue epsilon\nlet iota build theta delta\nfunction zeta blue build iota\nconst theta record item kappa\nlet west red status west"
  },
  {
   "ok": true,
   "value": "  return delta iota eta green\n  return iota value value\nexport entry\nfunction gamma green\nconst ready lambda eta check\n  if (status done\nlet omega red\nfunction theta north item east\nfunction epsilon eta\n}red theta east blue\nlet entry omega north ready\nexport record\nexport blue iota north\nfunction red item value done"
  },
  {
   "ok": true,
   "value": "  if (beta beta alpha\n  if (red east status\n  if (record epsilon done\nexport beta south blue\nlet zeta check entry\nfunction entry done check"
  },
  {
   "ok": true,
   "value": "let build omega\nconst epsilon\nlet theta check delta eta"
  },
  {
   "ok": true,
   "value": "  if (south record alpha\nfunction green build\n  return entry blue done\nfunction ready east north kappa\nfunction south\n  if (south record south north\nconst north record epsilon\nlet build theta\n  if (build record\n  if (eta\nexport done zeta"
  },
  {
   "ok": true,
   "value": "const north omega status green\nexport west\n  return test south iota\n}omega\nfunction iota"
  },
  {
   "ok": true,
   "value": "  if (done beta blue\nlet status delta item entry\nfunction gamma north west alpha\nlet value lambda kappa\nexport build\nexport beta record\n}west zeta blue"
  },
  {
   "ok": true,
   "value": "  if (build\nlet iota status\nfunction west value check blue\n  if (eta alpha south item\n}blue\nconst entry south alpha\n  if (theta value\nconst status\nexport ready eta status delta\nexport red entry\nfunction done south epsilon entry"
  },
  {
   "ok": true,
   "value": "  return east alpha kappa\n  if (entry ready done eta\nfunction green\n  return test eta"
  },
  {
   "ok": true,
   "value": "}check east\n}build epsilon west\nexport done kappa omega status\n}check\nconst red zeta lambda\nfunction south"
  },
  {
   "ok": true,
   "value": "function test kappa east\nlet iota south kappa epsilon\n}lambda gamma done item\n}status south gamma\nlet eta gamma\n  return south record red lambda\n  return north ready beta epsilon\n}test\n  if (red test item zeta"
  },
  {
   "ok": true,
   "value": "const green iota lambda\nfunction entry beta\n  if (gamma green\nfunction lambda beta\n}green\n  if (entry\n  return ready\nfunction lambda"
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
for(const path of ["src/dir1/file2.js","src/dir1/file3.js","src/dir1/file4.js","src/dir1/file5.js","src/dir1/file6.js","src/dir1/file7.js","src/dir8/file9.js","tests/file10.js","src/dir11/file12.js","src/dir11/file13.js","src/dir11/file14.js","src/dir11/file15.js","src/dir16/file17.js","src/dir16/file18.js"]) {text(path); text(await tools.read({path,limit:path.includes("vafupezovu.qara")?100:undefined}));}
})(fake);
