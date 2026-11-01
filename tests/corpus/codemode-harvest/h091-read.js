// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "Test delta east\nLambda iota check record west check\n# East epsilon done epsilon beta theta entry theta\n## Epsilon omega value\n1. Value alpha test north entry iota\n## Green item east item\nGamma kappa\nCheck epsilon\n# Alpha value zeta\n## Check east build\n# Beta epsilon north east west done\n# Test iota"
  },
  {
   "ok": true,
   "value": "1. Check delta ready\nEntry epsilon status\nBeta test beta east iota iota"
  },
  {
   "ok": true,
   "value": "1. Record check gamma check lambda test\n- Item status red item test\n# Omega red green west done done iota lambda"
  },
  {
   "ok": true,
   "value": "## Kappa east zeta\n## Item theta check red build item kappa green\n1. Blue kappa east beta\nEntry item theta\n- Record omega gamma done lambda alpha red\n- Blue delta kappa west done build\n## Zeta zeta lambda check item epsilon test\n## Epsilon west check omega delta green ready\n## Green entry theta"
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
const fs = ["DECISIONS.md","GOALS.md","dir1/README.md","dir2/README.md"];
const r = await Promise.all(fs.map(p=>tools.read({path:p,offset:null,limit:null})));
return r.map((x,i)=>`#### ${fs[i]}\n${x}`).join("\n\n");
})(fake);
