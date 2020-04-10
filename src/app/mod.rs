pub mod flags;
pub(crate) mod format;
pub mod header;
pub mod measurement;
pub mod parse;
pub mod sequence;
pub mod types;
pub mod gen {
    pub(crate) mod conversion;
    pub mod enums;
    pub mod variations {
        pub mod all;
        pub mod count;
        pub mod fixed;
        pub mod prefixed;
        pub mod ranged;
        pub mod variation;
    }
}
