// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export blue south ready done\n  if (item\n  if (eta north delta delta\nlet eta south north\nconst test south lambda check\nexport east item\nexport value\n}status\n  return zeta\n}south gamma\n}alpha west record west\nlet test green south red"
  },
  {
   "ok": true,
   "value": "  if (west south\n}lambda check\nlet gamma east status west\nexport red ready build\nlet south eta south\nconst delta\nfunction done item build ready\n  return gamma east theta\nexport south value record beta"
  },
  {
   "ok": true,
   "value": "  if (test red\nexport check zeta west\nconst theta blue record\nconst check item\n}blue entry red"
  },
  {
   "ok": true,
   "value": "}zeta south\n  return done theta south iota\nfunction entry north\nexport zeta east epsilon alpha\n  return red\n  return lambda red\n  if (test\n  if (delta eta theta west\nconst done\n}iota\nfunction blue delta\n  return epsilon status beta\nlet theta west green value"
  },
  {
   "ok": true,
   "value": "function beta omega\nexport lambda north\n  if (build status record status\n  if (entry kappa\n}lambda\nexport delta alpha item done\nfunction build kappa\nlet iota south\nfunction build gamma blue value\nlet status status done iota\nfunction done iota\n}ready\n  return theta"
  },
  {
   "ok": true,
   "value": "const blue\n}item status beta\nfunction kappa value value omega\nconst delta east\nfunction zeta omega\n  if (iota status entry\n}value item done\nexport delta\nexport omega green check east"
  },
  {
   "ok": true,
   "value": "  return south record ready\nlet east eta build\nconst eta test green check\nlet north record\n  return blue zeta zeta north\n  if (build red west\nlet ready\n}eta\nexport build omega\n  if (eta theta build item\nfunction value red\nfunction green record"
  },
  {
   "ok": true,
   "value": "  return red\nlet epsilon gamma entry omega\n}blue south blue"
  },
  {
   "ok": true,
   "value": "let entry gamma north delta\nfunction zeta entry\nfunction south item\nfunction test record\nfunction value\nlet check blue\nexport eta delta eta\n  if (entry"
  },
  {
   "ok": true,
   "value": "- Test beta blue north red delta lambda\n1. Item south west status\n# Done entry build south test theta\n- Green check south iota\nDelta entry entry red west omega iota\n- Ready delta test build done\n## Entry iota done build iota record blue\nIota ready lambda item epsilon\n# Record record eta kappa status zeta ready record\n- Beta north east north done\n# Theta beta\nBlue eta eta north red delta omega iota\n## Gamma beta red item beta west"
  },
  {
   "ok": true,
   "value": "# Gamma done west beta\n1. Green kappa entry alpha green done\n1. Kappa omega entry east\n1. Eta iota red entry west south theta\n- Omega item blue item record green test test\n1. Green theta blue gamma west green alpha\n- Delta north\n1. Red entry\nZeta test lambda gamma record entry\n- Omega east beta alpha beta\n# Item alpha ready\nEta red iota status entry zeta green\nStatus red epsilon check theta"
  },
  {
   "ok": true,
   "value": "  if (eta\n  return beta red\nlet north\nconst status east check done\n  return entry record status check"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "green theta delta done kappa record\nomega west epsilon west\nblue gamma blue\nbeta check status west\ngreen red ready entry value beta check theta\niota south beta\nkappa omega ready omega status lambda delta red\niota theta west item\ndelta check east kappa north entry kappa\nzeta omega done beta iota beta\ndone zeta zeta check\nitem blue"
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
await Promise.allSettled(["dir1/src/dir2/file3.js","dir1/src/dir4/file5.js","dir1/src/dir4/file6.js","dir1/src/dir7/file8.js","dir1/src/core/file8.js","dir1/src/dir7/file9.js","dir1/src/dir2/file10.js","dir1/src/dir2/file11.js","dir1/src/dir12/file13.js","dir14/dir15/README.md","dir14/file16.daz","dir1/src/file17.js"].map(async path => text({path,contents:await tools.read({path})})));
text(await tools.bash({command:"dutu --gopop-givu=titoni <<'JS'\nbanazi {pavuDapaReze,pemamupReze} bidi 'dutu:fs';\nqez(rolek f of pemamupReze('ropapet').gipere(f=>f.makaMavi('.qub'))){rolek b=pavuDapaReze('ropapet/'+f); rolek j=FUVE.natel(b.visirida(20,20+b.pavuUQal32LE(12)));vozesag.qot('\\n'+f);qez(rolek i of j.vobemo[j.dogiv||0].ronup||[])vozesag.qot(FUVE.vurokivun({fazu:j.ronup[i].fazu,refomi:j.ronup[i].refomi}));if(j.gikipokipe)vozesag.qot('gikipokipe:',j.gikipokipe.zil(a=>a.fazu));}\nJS","pizofom":null}));
})(fake);
