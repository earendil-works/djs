// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "## Status alpha east red build check check\n1. Gamma check status iota entry\n# North done ready iota lambda east alpha\n- Lambda west theta entry\n1. Eta test kappa status check\n1. West check omega\n1. Ready status status epsilon\n1. Red status delta\n## South build lambda green build delta\nReady eta\n- Value lambda epsilon kappa epsilon delta\n- Epsilon gamma epsilon item kappa\nCheck value west lambda ready blue"
  },
  {
   "ok": true,
   "value": "1. Item green lambda status alpha green eta\n# Ready entry north\n# Delta omega build test red\n- Item value south ready ready iota\nWest west alpha\n## Red omega theta delta\n## Record zeta zeta build alpha\nKappa build\n# Eta west gamma done\n# Beta blue"
  },
  {
   "ok": true,
   "value": "record north done entry lambda green south\nomega iota alpha east north eta epsilon iota\nzeta west west delta value 3111\ndelta east beta zeta lambda alpha\ndelta epsilon east lambda eta lambda ready\ndone green\ndelta record value blue green omega 8629\neast build omega item lambda\neast value omega\ntest check eta omega item\nvalue green build eta test theta\nzeta status item status"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "south value blue south\nlambda omega entry lambda delta\nalpha gamma east gamma check status\nalpha kappa entry south record test status\nnorth blue value\niota beta theta eta delta zeta theta iota\nentry check eta omega 1991\ntest lambda value theta gamma value status\nwest kappa kappa zeta\ngreen theta check green eta south done delta\neta test 6898"
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
const results = await Promise.allSettled([
tools.read({path:"/zase/qide/qin1/qin2/qin3/qin4/geze5.md"}),
tools.read({path:"/zase/qide/qin1/qin6/qin4/qin7/geze8.md"}),
tools.read({path:"qin9/qin10/geze11.vil"}),
tools.bash({command:"qaz bigagi --tepir; qaz rapiga -v; gh kobavano fuvi pubepug-gogaf-gafurov.vil","tamagik":60})
]); for (let i=0;i<results.length;i++) text({i,...results[i]});
})(fake);
