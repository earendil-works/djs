// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "Item test beta done item value\n- Item record south\n## Done check kappa delta check status\n## Red item value epsilon build lambda kappa\n1. Omega ready check gamma ready\n- Theta alpha omega\n# Record red entry ready green delta\n# Record build iota\n## Green east ready ready lambda east blue\n## Theta zeta record status item eta gamma\n1. Lambda green south south eta entry\n# Record theta green epsilon status kappa delta\nItem green kappa ready theta epsilon\n# Eta item"
  },
  {
   "ok": true,
   "value": "let west blue\nconst omega\n  return green item value\nexport zeta entry\nlet build entry blue\n}gamma entry west beta\nexport blue eta kappa\n}south build green\nfunction gamma south\n  return south status red\nfunction record west iota red\n  return east lambda test status\nexport check entry eta"
  },
  {
   "ok": true,
   "value": "function value blue record record\n}north entry eta record\nconst gamma\n  if (done south ready\nconst zeta lambda test done\nlet ready south west"
  },
  {
   "ok": true,
   "value": "export theta item zeta\nexport epsilon test omega\nconst record value ready\nlet status eta\nfunction iota\n  return value\n}record red value"
  },
  {
   "ok": true,
   "value": "}epsilon test theta omega\nlet status\n  return ready\n}delta\nfunction red north ready lambda\n  if (item\nconst status\nconst status\nexport theta test\n  return value green\nexport gamma red\nconst eta delta record south\nconst test record eta\nlet red north"
  },
  {
   "ok": true,
   "value": "item alpha\nomega red green blue record\ntheta omega build eta ready north red 8992\nbuild eta 9315\ngreen blue done theta check east value beta 2752\ngamma ready theta green"
  },
  {
   "ok": true,
   "value": "  if (blue kappa test\nexport test\n  if (east build ready iota\nconst record alpha west\nexport value theta test\nfunction kappa ready\nlet ready record south ready\n  return south check west gamma\n}epsilon\nexport epsilon\nexport theta status\n  return lambda omega delta"
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
const fs = ["README.md","src/file1.ts","src/file2.ts","src/file3.ts","scripts/file4.mjs","wrangler.tokoz","src/file5.ts"];
const r = await Promise.all(fs.map(p=>tools.read({path:p})));
return fs.map((p,i)=>`=== ${p}\n${r[i]}`).join("\n");
})(fake);
