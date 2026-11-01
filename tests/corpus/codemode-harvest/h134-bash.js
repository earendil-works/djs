// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "alpha iota blue south\nlambda omega ready done alpha north\nnorth record green status entry item build\ntheta beta entry beta\ntest delta gamma status 4359\nblue zeta north eta gamma"
  },
  {
   "ok": true,
   "value": "value ready value build check alpha record delta\neta delta entry omega\nwest omega value kappa"
  },
  {
   "ok": true,
   "value": "epsilon item red record theta entry lambda\nentry delta\nsouth item\niota west kappa status theta check kappa"
  },
  {
   "ok": true,
   "value": "north iota ready entry west epsilon red iota 5954\ndone check beta ready blue done omega\ngamma value south red beta lambda west value 9296\nlambda blue east build omega\nbuild delta north\nbuild beta east north status\neta ready\nkappa eta east 3196\nalpha iota\nrecord value epsilon green check beta south\nomega test\ndelta beta test alpha 1352\nblue north blue iota\nepsilon blue value test beta build status"
  },
  {
   "ok": true,
   "value": "alpha ready done blue test entry\nbuild alpha gamma record eta alpha south 8563\nvalue check gamma south zeta west\niota status epsilon 7188\neta ready epsilon lambda blue kappa\nomega test blue kappa 9131\neast eta kappa north\nred build lambda west\nomega north value\ngamma red alpha 2536\nzeta omega eta\nomega zeta lambda 2308"
  },
  {
   "ok": true,
   "value": "record iota red lambda north omega value 2676\nlambda check beta east red lambda alpha\nready entry zeta gamma green omega south done\nred lambda 4402\ngreen lambda west omega\ncheck test south iota beta theta 7641\nlambda gamma east delta west value\nitem lambda entry beta\nbeta eta alpha green beta build\nzeta south"
  },
  {
   "ok": true,
   "value": "north record test value record delta theta epsilon\nblue status record red north check epsilon lambda 2260\nwest south omega\nzeta kappa omega zeta eta\nblue eta item west delta red beta\nepsilon test status green ready lambda 6891\nbeta eta lambda lambda blue\nnorth eta alpha iota gamma\nomega west beta north 6131"
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
const base = "packages/dir1/src/";
const files = ["core/dir2/file3.ts","core/dir2/file4.ts","core/dir2/file5.ts","core/file6.ts","dir2/dir7/index.ts","dir2/dir7/file8.ts","core/dir9/file10.ts"];
const res = await Promise.all(files.map(f => tools.bash({command: `qos qogo -U2 -- ${base}${f}`, timeout: null})));
res.forEach(r => text(r.output));
})(fake);
