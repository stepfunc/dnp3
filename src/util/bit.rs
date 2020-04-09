pub trait BitTest {
    const BIT_0: u8 = 0b0000_0001;
    const BIT_1: u8 = 0b0000_0010;
    const BIT_2: u8 = 0b0000_0100;
    const BIT_3: u8 = 0b0000_1000;
    const BIT_4: u8 = 0b0001_0000;
    const BIT_5: u8 = 0b0010_0000;
    const BIT_6: u8 = 0b0100_0000;
    const BIT_7: u8 = 0b1000_0000;

    fn bit_0(self) -> bool;
    fn bit_1(self) -> bool;
    fn bit_2(self) -> bool;
    fn bit_3(self) -> bool;
    fn bit_4(self) -> bool;
    fn bit_5(self) -> bool;
    fn bit_6(self) -> bool;
    fn bit_7(self) -> bool;
}

impl BitTest for u8 {
    fn bit_0(self) -> bool {
        self & Self::BIT_0 != 0
    }

    fn bit_1(self) -> bool {
        self & Self::BIT_1 != 0
    }

    fn bit_2(self) -> bool {
        self & Self::BIT_2 != 0
    }

    fn bit_3(self) -> bool {
        self & Self::BIT_3 != 0
    }

    fn bit_4(self) -> bool {
        self & Self::BIT_4 != 0
    }

    fn bit_5(self) -> bool {
        self & Self::BIT_5 != 0
    }

    fn bit_6(self) -> bool {
        self & Self::BIT_6 != 0
    }

    fn bit_7(self) -> bool {
        self & Self::BIT_7 != 0
    }
}
