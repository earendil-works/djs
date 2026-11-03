// Reference cycles through what is in flight: an activation whose argument holds its own
// result promise, and a promise chain kept on an object that a pending reaction reads.
const state = { runs: [] };
async function run(s, n) {
  await later(n);
  s.runs.push(n);
  return s.runs.length;
}
state.task = run(state, 1);
state.chain = state.task.then((k) => state.runs.concat(k));
async function main() {
  print(await state.task, JSON.stringify(await state.chain));
  state.task = run(state, 2);
  print(await state.task, state.runs.join());
}
main();
// A parameter past the arguments given, assigned before an await and shared.
const shared = [];
async function late(a, b) {
  b = shared;
  await later(a);
  return b.length;
}
const lateRun = late(3);
shared.push(1);
lateRun.then((n) => print("late", n));
// An argument object that is also its parameter's binding.
async function both(list) {
  await later(list.length);
  return list.join("+");
}
both(["x", "y"]).then((s) => print("both", s));
