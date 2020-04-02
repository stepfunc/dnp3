pub fn log_items<T, V, I>(level: log::Level, iter: T)
where
    T: Iterator<Item = (V, I)>,
    V: std::fmt::Display,
    I: std::fmt::Display,
{
    for (v, i) in iter {
        log::log!(level, "index: {} {}", i, v);
    }
}
