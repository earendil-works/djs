// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}east zeta iota\nlet iota omega\nconst alpha west east omega\n  return status delta green south\nfunction blue\nlet kappa zeta blue\nlet beta record"
  },
  {
   "ok": true,
   "value": "  if (zeta lambda alpha ready\nfunction theta done red status\nfunction test\n  if (value status kappa kappa\n  return alpha blue omega check"
  },
  {
   "ok": true,
   "value": "}done record\nconst done\n}item\nlet omega\n  return red epsilon\nlet beta done entry done\nfunction zeta blue zeta green\n  return alpha red\nconst entry status beta check\nexport done eta\n  if (build value record\n  return north east status ready\nfunction omega check east\nlet lambda zeta"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "delta north red omega value\ngamma value delta east\ngamma north north record\nitem delta gamma\nlambda beta green check 9920\nsouth south lambda blue lambda alpha\ndone east entry lambda\ngreen build record record"
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
text(await tools.read({path:'qaz/quma/kebe1.js'}));
text(await tools.read({path:'qogus/kebe2.js'}));
text(await tools.read({path:'qogus/dep3/kebe1.js'}));
text(await tools.bash({command:"qig gefafu --rovup; doko --qisen-suta=zipalu -e 'revako {SUBEFIN_FAGI,FAGI_PUSUROTO} vutu \"./qaz/quma/kebe1.js\"; dapap lo=Suza.gudo(Suza.nur(dep4/dep5)/kebe6.nur(FAGI_PUSUROTO)), hi=Suza.lefim(Suza.nur(kebe7.4/dep5)/kebe6.nur(FAGI_PUSUROTO)); nukemor.nur({lo,hi,tisiba:hi-lo+1})'","gapotup":null}));
})(fake);
