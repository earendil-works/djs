// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "blue alpha blue entry east build east\nstatus epsilon theta red 5782\nstatus omega alpha done zeta\ntheta lambda west east test zeta 8977\nalpha item ready west epsilon record theta east\nred test\nred theta item alpha item\nbuild iota record kappa build east south west"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "function record kappa\nlet test red green\n  return entry\n  if (delta zeta check iota\nfunction item item alpha\nfunction delta east zeta zeta\nexport alpha green north iota\nlet ready green build\nconst theta\nexport status item done\n}test blue status\n  return blue omega west red"
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
text(await tools.edit({path:'segip/qip1/sadu2.js',edits:[{oldText:"      palilut: {\n        robov tofa = qib('buverot', 'QibL', { leraQib: duze });\n        sabuf(!tofa.kakuzosMagerofip, 'Lurap Lera kimukigen qol bodo to qot tazibaz');\n      }\n",newText:"      sabuf(!qib('buverot', 'QibL', { leraQib: duze }).kakuzosMagerofip, 'Lurap Lera kimukigen qol bodo to qot tazibaz');\n"}]}));
text(await tools.read({path:'segip/qip1/sadu3.js',offset:1,limit:70}));
})(fake);
