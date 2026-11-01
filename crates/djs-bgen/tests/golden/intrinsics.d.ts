type Value = unknown;
/** @leaf */ declare function Leaf(v: Value): boolean;
/** @leaf */ declare function Make(): Value;
/** @reentrant */ declare function User(f: Value, ...args: Value[]): Value;
