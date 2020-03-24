pub mod parse;
pub mod types;
pub mod gen {
    pub mod enums;
    pub mod variations {
        pub mod all;
        pub mod count;
        pub mod fixed;
        pub mod gv;
        pub mod prefixed;
        pub mod ranged;
    }
}
