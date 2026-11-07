//! djs-libm is a copy of libm 0.2.16 (D3174): every function gives the same bits as libm's on
//! special values, on values around the thresholds the implementations branch on, and on
//! many random ones (random bit patterns, which are mostly huge or tiny, and random values of
//! moderate size).

/// xorshift64*: a fixed seed, so a failure reproduces.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A unit-interval value.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// One of several distributions in turn: any bit pattern, moderate values, values around
    /// a power of two, small integers and halves, tiny values.
    fn value(&mut self, i: usize) -> f64 {
        let sign = if self.next() & 1 == 0 { 1.0 } else { -1.0 };
        match i % 6 {
            0 => f64::from_bits(self.next()),
            1 => sign * self.unit() * 10.0,
            2 => sign * self.unit() * 1000.0,
            3 => sign * libm::ldexp(1.0 + self.unit(), (self.next() % 128) as i32 - 64),
            4 => sign * ((self.next() % 400) as f64 / 2.0 - 100.0),
            _ => sign * libm::ldexp(self.unit(), -((self.next() % 1100) as i32)),
        }
    }
}

/// Values every function sees: the specials, the extremes, and the thresholds the libm
/// implementations test (exp/sinh/cosh overflow and underflow, the pi/4 multiples of the
/// trig reductions, 1 for the logs and inverse hyperbolics, 2^-28 and 2^28 for asinh...).
fn specials() -> Vec<f64> {
    let mut v = vec![
        0.0,
        f64::INFINITY,
        f64::NAN,
        f64::from_bits(0x7ff0_0000_0000_0001), // signaling NaN
        f64::from_bits(0x7fff_ffff_ffff_ffff),
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        f64::from_bits(1),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        f64::MAX,
        f64::EPSILON,
        1.0,
        0.5,
        1.5,
        2.0,
        3.0,
        10.0,
        100.0,
        1e-300,
        1e300,
        1e16,
        4503599627370496.0, // 2^52
        4503599627370495.5,
        9007199254740992.0, // 2^53
        709.782712893384,
        709.7827128933841,
        710.4758600739439,
        -745.1332191019411,
        -745.1332191019412,
        -708.3964185322641,
        22.0,
        0.5493061443340548, // atanh(0.5)
        0.881373587019543,  // asinh(1)
        core::f64::consts::PI,
        core::f64::consts::FRAC_PI_2,
        core::f64::consts::FRAC_PI_4,
        core::f64::consts::E,
        core::f64::consts::LN_2,
        core::f64::consts::SQRT_2,
        1e22,
        1e308,
        0.9999999999999999,
        1.0000000000000002,
        3.725290298461914e-9,  // 2^-28
        268435456.0,           // 2^28
        1.4901161193847656e-8, // 2^-26
        7.450580596923828e-9,  // 2^-27
        2.0f64.powi(-1022),
        2.0f64.powi(-1074),
        2.0f64.powi(1023),
        1.0e-5,
        0.25,
        0.75,
        0.9,
        0.1,
        0.3,
    ];
    // Multiples of pi/4 and their neighbours, up to where rem_pio2 switches to the large
    // reduction (2^20 pi/2) and past it.
    for k in [
        1.0, 2.0, 3.0, 4.0, 5.0, 8.0, 100.0, 1e5, 1e6, 1e10, 1e20, 1e200,
    ] {
        let x = k * core::f64::consts::FRAC_PI_4;
        v.extend([
            x,
            f64::from_bits(x.to_bits() + 1),
            f64::from_bits(x.to_bits() - 1),
        ]);
    }
    v.extend([1647099.3291652855, 1647099.0 * core::f64::consts::FRAC_PI_2]);
    let neg: Vec<f64> = v.iter().map(|x| -x).collect();
    v.extend(neg);
    v
}

const RANDOM: usize = 200_000;

fn same(name: &str, args: &[f64], ours: f64, theirs: f64) {
    assert!(
        ours.to_bits() == theirs.to_bits(),
        "{name}{args:?}: djs-libm {ours:e} ({:#018x}), libm {theirs:e} ({:#018x})",
        ours.to_bits(),
        theirs.to_bits()
    );
}

fn unary(name: &str, ours: fn(f64) -> f64, theirs: fn(f64) -> f64) {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ name.len() as u64);
    let mut inputs = specials();
    inputs.extend((0..RANDOM).map(|i| rng.value(i)));
    for x in inputs {
        same(name, &[x], ours(x), theirs(x));
    }
}

fn binary(name: &str, ours: fn(f64, f64) -> f64, theirs: fn(f64, f64) -> f64) {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d ^ name.len() as u64);
    let sp = specials();
    let mut inputs: Vec<(f64, f64)> = Vec::new();
    for &x in &sp {
        for &y in &sp {
            inputs.push((x, y));
        }
    }
    inputs.extend((0..RANDOM).map(|i| (rng.value(i), rng.value(i / 6 + i))));
    for (x, y) in inputs {
        same(name, &[x, y], ours(x, y), theirs(x, y));
    }
}

macro_rules! unary_tests {
    ($($f:ident)*) => {$(
        #[test]
        fn $f() {
            unary(stringify!($f), djs_libm::$f, libm::$f);
        }
    )*};
}

macro_rules! binary_tests {
    ($($f:ident)*) => {$(
        #[test]
        fn $f() {
            binary(stringify!($f), djs_libm::$f, libm::$f);
        }
    )*};
}

unary_tests!(
    trunc floor ceil sqrt fabs sin cos tan asin acos atan sinh cosh tanh asinh acosh atanh exp
    expm1 log log2 log10 log1p cbrt
);

binary_tests!(fmod atan2 pow hypot);

#[test]
fn ldexp() {
    let mut rng = Rng(7);
    let mut inputs = specials();
    inputs.extend((0..RANDOM).map(|i| rng.value(i)));
    let ns = [
        0,
        1,
        -1,
        2,
        52,
        53,
        -52,
        -53,
        1023,
        1024,
        -1022,
        -1023,
        -1074,
        -1075,
        2046,
        -2098,
        i32::MAX,
        i32::MIN,
    ];
    for x in inputs {
        for n in ns {
            same(
                "ldexp",
                &[x, n as f64],
                djs_libm::ldexp(x, n),
                libm::ldexp(x, n),
            );
            same(
                "scalbn",
                &[x, n as f64],
                djs_libm::scalbn(x, n),
                libm::scalbn(x, n),
            );
        }
        let n = (rng.next() % 4400) as i32 - 2200;
        same(
            "ldexp",
            &[x, n as f64],
            djs_libm::ldexp(x, n),
            libm::ldexp(x, n),
        );
    }
}

/// `pow` on the cases the spec singles out (D1150 handles some before calling it) and on
/// integer exponents, where C's pow is exact.
#[test]
fn pow_integer_exponents() {
    let mut rng = Rng(11);
    for i in 0..RANDOM {
        let x = rng.value(i);
        let y = (rng.next() % 200) as f64 - 100.0;
        same("pow", &[x, y], djs_libm::pow(x, y), libm::pow(x, y));
    }
}

/// `fma`, which cbrt uses (the hardware instruction where libm picks it, else libm's soft one).
#[test]
fn fma() {
    let mut rng = Rng(13);
    let sp = specials();
    for (i, &x) in sp.iter().enumerate() {
        for &y in &sp {
            let z = sp[(i * 7) % sp.len()];
            same(
                "fma",
                &[x, y, z],
                djs_libm::fma(x, y, z),
                libm::fma(x, y, z),
            );
        }
    }
    for i in 0..RANDOM {
        let (x, y, z) = (rng.value(i), rng.value(i + 1), rng.value(i + 2));
        same(
            "fma",
            &[x, y, z],
            djs_libm::fma(x, y, z),
            libm::fma(x, y, z),
        );
        // A product that nearly cancels the addend, where fusing matters.
        let z = -(x * y);
        same(
            "fma",
            &[x, y, z],
            djs_libm::fma(x, y, z),
            libm::fma(x, y, z),
        );
    }
}
