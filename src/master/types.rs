pub struct ClassScan {
    pub class1: bool,
    pub class2: bool,
    pub class3: bool,
    pub class0: bool,
}

impl ClassScan {
    pub fn new(class1: bool, class2: bool, class3: bool, class0: bool) -> Self {
        Self {
            class1,
            class2,
            class3,
            class0,
        }
    }

    pub fn integrity() -> Self {
        Self::new(true, true, true, true)
    }
}
