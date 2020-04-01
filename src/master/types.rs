pub struct ClassScan {
    pub class1: bool,
    pub class2: bool,
    pub class3: bool,
    pub class0: bool,
}

impl ClassScan {
    pub fn integrity() -> Self {
        Self {
            class1: true,
            class2: true,
            class3: true,
            class0: true,
        }
    }
}
