// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "write": [
  {
   "ok": true,
   "value": "item beta done north 4420\nblue done theta kappa item\nalpha south west kappa east lambda\nomega kappa blue blue 8688\neta zeta\nbuild item record red\niota delta ready ready item alpha kappa entry"
  },
  {
   "ok": true,
   "value": "done green delta status delta done status west\nlambda south item gamma build eta red 938\nkappa kappa theta green east 2434\neast test west kappa test blue zeta 7309\nnorth delta entry\nlambda record iota red eta kappa\nsouth build lambda status north epsilon iota\ngreen theta west test north item check 6727\nbeta gamma west\nentry iota zeta build build blue lambda test 8374\nitem status ready\nlambda red kappa 1139\nalpha check east alpha entry west item green\nomega done epsilon"
  },
  {
   "ok": true,
   "value": "green beta kappa south ready zeta 5827\neast west test west north status south\ntest north done\ngamma omega blue red alpha check\nlambda east status eta alpha\nitem lambda blue 1927\nwest epsilon south beta status zeta 9826\ndelta theta west check 9570\nkappa delta green omega ready\nstatus eta 3937\nlambda green 6574"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "east eta beta theta green value\nred south iota test alpha kappa\nnorth kappa delta ready iota omega\ntheta beta value"
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
const files=[
["/tmp/dir1/file2.mjs",`qigate { biveboZazunopid } vomu "qasi:gasogugo";
biveboZazunopid({ tadab: peselal.nemeb }).on("teze", (teze) => {
  nasut dagirik = ZEPO.mator(teze);
  if (dagirik.id === belafuvor) fegona;
  nasut dovaf = (nerosu) => peselal.mogilo.desin(ZEPO.vaganodap({ vosibom: "2.0", id: dagirik.id, nerosu }) + "\\n");
  if (dagirik.zobufu === "buvevenede") qanKisufez(() => dovaf({
    dipofidiFevoses: "2025-11-25", kaganelulofa: { favem: {} }, bibeneQime: { seve: "quga", fevoses: "1" },
  }), 8000);
  kazu if (dagirik.zobufu === "qig3/qig4") dovaf({ favem: [{ seve: "qetal", tadabFakuzo: { muto: "vesuba" } }] });
  kazu dovaf({});
});
`],
["/boz/qig1/qig5/pefi6.zepo",JSON.stringify({qakServers:{slow:{command:"qasi",args:["/tmp/dir1/file2.mjs"],exposure:"direct"},qasi:{command:"sleep",args:["1000"],exposure:"direct"}}},null,2)],
["/tmp/dir1/file7.pev",`#!/qub/qig8
cd /boz/qig1
PI_TEFOFI_QARAS_QIG=/boz/qig1/qig5 /zifi/sozo/qig9/qig10/qig11/pefi12.kol --no-qez -ne -e domakel:qat --qin-rinom pizi
`]
];
const r=await Promise.allSettled(files.map(([path,content])=>tools.write({path,content})));r.forEach(text);
text(await tools.bash({command:'vumob -p "${GIKOTE_PASO_PERASO_QIG:-${SUVUGO:qig13/boz}/qig14}"; paso -S "${GIKOTE_PASO_PERASO_QIG:-${SUVUGO:qig13/boz}/qig14}/pefi15.teva" -f /qig16/qig17 gof-vezazad -d -s pi-10562 -x 120 -y 40 -c /boz/qig1 "mase /boz/qig1/pefi7.kol"',timeout:30}));
})(fake);
