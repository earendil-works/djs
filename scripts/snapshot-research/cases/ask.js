// Human in the loop: the script waits (possibly for days) on an approval.
const plan = { steps: ["build", "test", "deploy"], target: "staging" };
const answer = await tools.ask({ question: `Deploy to ${plan.target}?`, options: ["yes", "no"] });
if (answer.choice !== "yes") return "cancelled";
const out = await tools.shell({ cmd: "make deploy TARGET=" + plan.target });
console.log(out.stdout.split("\n").slice(-3).join("\n"));
return { ok: out.code === 0, steps: plan.steps.length };
