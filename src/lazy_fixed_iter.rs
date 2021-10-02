macro_rules! lazy_fixed_iter {
    (move || $e:expr) => {
       iter::once_with(move || $e)
    };
    ($e:expr) => {
       iter::once_with(|| $e)
    };
    (move || $e:expr, $($es:expr),+) => {
        lazy_fixed_iter!(move || $e)
            .chain(lazy_fixed_iter!(move || $($es), +))
    };
    ($e:expr, $($es:expr),+) => {
        lazy_fixed_iter!($e)
            .chain(lazy_fixed_iter!($($es), +))
    };
}
