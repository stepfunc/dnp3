pub fn log_items<T, V>(level: log::Level, iter: T) where T : Iterator<Item = (V, u16)>, V : std::fmt::Display {
    for (v, i) in iter {
        log::log!(level, "index: {} {}", i, v);
    }
}