// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "delta west lambda 9277\nlambda item done 6921\nsouth lambda\nbuild entry\nlambda record zeta value\ngamma delta zeta\nvalue ready epsilon beta north 9030\nomega epsilon\nlambda value zeta south zeta theta ready delta\nbuild alpha epsilon south iota beta 7726"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "south test alpha blue test record entry 9598\nwest record epsilon south status\nomega beta theta lambda build\nrecord lambda omega item done blue\nitem eta south epsilon south alpha iota done\nwest record status north value epsilon epsilon beta\nepsilon south\nbuild east check value\nalpha west beta\nready value green north 9055\ngreen check zeta check green 4607\nblue build zeta zeta\nbeta theta east north build"
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
const html = await tools.read({ path: "/vek/muzo1.qive", offset: null, limit: 5000 });

const res = await tools.bash({ command: "wc -c < /vek/muzo1.qive", timeout: null });
return { len: html.length, file: res.output, tail: html.slice(-200) };
})(fake);
