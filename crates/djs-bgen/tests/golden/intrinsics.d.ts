type Value = unknown;
/** @leaf */ declare function Leaf(v: Value): boolean;
/** @leaf */ declare function Make(): Value;
/** @reentrant */ declare function User(f: Value, ...args: Value[]): Value;
/** @reentrant */ declare function Get(o: Value, k: Value): Value;
/** @reentrant */ declare function LengthOfArrayLike(o: Value): f64;
/** @leaf */ declare function NewError(msg: Str): Value;
