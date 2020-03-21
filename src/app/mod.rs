pub mod header;
pub mod parser;
pub mod range;
pub mod types;
pub mod gen {
    pub mod enums;
    pub mod variations {
        pub mod allobjs;
        pub mod fixed;
        pub mod gv;
        pub mod ranged;
    }
}
