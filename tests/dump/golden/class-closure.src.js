// A class made by a factory, closing over its variable; instances in flight.
function make(prefix) {
  return class Tagged {
    constructor(v) { this.v = v; }
    async tag() { return prefix + (await later(this.v)); }
  };
}
const T = make("#");
const t = new T(4);
t.tag().then(print);
