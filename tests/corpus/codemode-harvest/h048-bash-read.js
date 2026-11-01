// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "entry theta build 2841\nepsilon kappa test\nepsilon blue status north\ngamma west south value status record iota north\niota iota value west iota check\ntest red south north status south\nbeta eta lambda check blue south green 9888\ntest iota build iota zeta west\nzeta red test blue item record\nblue lambda eta kappa omega east 7715\nred test delta ready\ntest status delta green theta beta omega value"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "eta delta delta lambda\nwest omega\nready kappa blue\ndone item\neast entry omega south blue eta\ntest eta\ncheck west test iota omega\nsouth zeta"
  },
  {
   "ok": true,
   "value": "eta beta gamma west record item beta\ngreen value south 313\ngreen iota ready lambda ready iota\nomega zeta check record south gamma item theta\nready value\nepsilon eta item lambda iota 121"
  },
  {
   "ok": true,
   "value": "epsilon omega\nbeta north test 3677\nepsilon alpha ready eta zeta build beta kappa\nzeta build green 8085\nalpha alpha zeta south 1870\nred status entry record blue west done\nred west"
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
text(await tools.bash({command:"wc -l /vak/kak1/beme2.peti",timeout:null}));
const part = await tools.read({path:"/vak/kak1/beme2.peti",offset:1,limit:80});
text({characters:part.length,end:part.slice(-180)});
store("reportPart1",part);
const part2=await tools.read({path:"/vak/kak1/beme2.peti",offset:81,limit:80});
text({characters:part2.length,end:part2.slice(-180)});
store("reportPart2",part2);
const part3=await tools.read({path:"/vak/kak1/beme2.peti",offset:161,limit:80});
text({characters:part3.length,end:part3.slice(-180)});
store("reportPart3",part3);
})(fake);
