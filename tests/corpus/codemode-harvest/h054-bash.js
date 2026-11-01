// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "build iota gamma theta lambda\nrecord kappa done epsilon check\nvalue status east delta record east build kappa\ntheta test\nrecord record east test ready"
  },
  {
   "ok": true,
   "value": "src/file6.ts:83: entry ready\nsrc/file2.ts:208: iota entry beta\nsrc/file2.ts:366: south east\nsrc/file9.ts:142: omega blue alpha omega west epsilon\nsrc/file6.ts:321: omega eta omega"
  },
  {
   "ok": true,
   "value": "src/file6.ts:126: record beta green\nsrc/file1.ts:167: eta epsilon done\nsrc/file1.ts:176: theta epsilon\nsrc/file7.ts:12: entry green status lambda epsilon theta\nsrc/file2.ts:95: build entry green north green\nsrc/file2.ts:204: test status entry\nsrc/file5.ts:64: theta alpha entry\nsrc/file6.ts:280: alpha alpha build gamma gamma\nsrc/file7.ts:233: beta entry east kappa eta north\nsrc/file6.ts:352: record entry record\nsrc/file9.ts:136: zeta beta delta blue alpha\nsrc/file7.ts:295: alpha iota check\nsrc/file3.ts:249: entry entry\nsrc/file8.ts:75: red kappa green blue zeta blue"
  },
  {
   "ok": true,
   "value": "src/file1.ts:316: iota build\nsrc/file1.ts:114: south beta theta\nsrc/file3.ts:114: red zeta south done\nsrc/file5.ts:91: omega east iota lambda west\nsrc/file2.ts:365: eta value north\nsrc/file7.ts:68: blue epsilon done alpha zeta lambda\nsrc/file4.ts:122: value test\nsrc/file7.ts:192: south kappa\nsrc/file7.ts:396: item gamma red south west\nsrc/file8.ts:248: check red ready omega theta red\nsrc/file7.ts:397: record check done build\nsrc/file9.ts:387: blue red theta north status\nsrc/file6.ts:152: delta epsilon ready test\nsrc/file3.ts:291: done ready entry"
  },
  {
   "ok": true,
   "value": "check kappa east\ndelta entry green alpha 8052\nready beta red omega\nnorth iota green ready status south alpha 7172\niota ready item red east status"
  },
  {
   "ok": true,
   "value": "east ready delta 4636\nepsilon beta build item 8715\ndone south\nstatus omega blue theta eta entry 7348\nbuild kappa build gamma beta\ntheta north zeta epsilon 1599\ngamma check build 9664"
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
const ca = "packages/dir1/";
const cmds = {
  docs: `qob qoro -U2 -- ${ca}LATAPOPER.md ${ca}docs/file2.md ${ca}examples/dir3/README.md ${ca}src/index.ts ${ca}src/core/dir3/index.ts`,
  childIter: `rg -n "bupoFedugavib\\.mipisezu" ${ca}src/dir4/dir5/file6.ts`,
  oldLookup: `rg -n "qedSapapugadoKekiKilumirugi|qedKekiKilumirugi\\(" bagolupe --lemu '!**/qara_gesezag/**' --lemu '*.ts' | tife -v neri/`,
  rendererTypes: `rg -n "ligudoper KekiVenanafor|qiro KekiVenanafor" bagolupe --lemu '*.ts' --lemu '!**/qara_gesezag/**'`,
  testsLen: `wc -l ${ca}test/file7.ts ${ca}test/file8.ts ${ca}test/dir9/dir10/file11.ts`,
  pofa: `qag ${ca}test/dir9/dir10/file11.ts`,
};
const keys = Object.keys(cmds);
const out = await Promise.all(keys.map(k => tools.bash({command: cmds[k], timeout: null})));
keys.forEach((k, i) => text(`=== ${k}\n${out[i].output}`));
})(fake);
