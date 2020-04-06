use crate::app::parse::prefix::Prefix;
use crate::app::parse::traits::FixedSize;

pub fn log_count_of_items<T, V>(level: log::Level, iter: T)
where
    T: Iterator<Item = V>,
    V: std::fmt::Display,
{
    for x in iter {
        log::log!(level, "{}", x);
    }
}

pub fn log_indexed_items<T, V, I>(level: log::Level, iter: T)
where
    T: Iterator<Item = (V, I)>,
    V: std::fmt::Display,
    I: std::fmt::Display,
{
    for (v, i) in iter {
        log::log!(level, "index: {} {}", i, v);
    }
}

pub fn log_prefixed_items<T, V, I>(level: log::Level, iter: T)
where
    T: Iterator<Item = Prefix<I, V>>,
    V: FixedSize + std::fmt::Display,
    I: FixedSize + std::fmt::Display,
{
    for x in iter {
        log::log!(level, "index: {} {}", x.index, x.value);
    }
}
