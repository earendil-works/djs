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
