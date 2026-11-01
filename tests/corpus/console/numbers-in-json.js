// Inside JSON, non-finite numbers become null and -0 becomes 0.
console.log([NaN, Infinity, -Infinity, -0, 0.1 + 0.2, 1 / 3, 5e-324, 1.7976931348623157e308]);
console.log({ nan: NaN, negzero: -0 });
console.log([123456789012345680000, 0.000001, 1e-7, 100, 1e100]);
console.log([new Number(1), new String("s"), new Boolean(false)]);
console.log(new Number(-0), new String("boxed"), new Boolean(true));
