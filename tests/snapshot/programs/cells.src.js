// A frame and a closure sharing a variable that is still in its TDZ at the await.
// At idle point 1: h waits on later(1); `late` is in its TDZ, shared with `get`: the
// snapshot keeps the cell before `let late` runs.
async function h() {
  const get = () => late;
  let early;
  try {
    get();
  } catch (e) {
    early = e.name;
  }
  const v = await later(1);
  let late = v * 2;
  return early + " " + get();
}
h().then(print);
