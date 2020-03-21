//  _   _         ______    _ _ _   _             _ _ _
// | \ | |       |  ____|  | (_) | (_)           | | | |
// |  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |
// | . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |
// | |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|
// |_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)
//                                           __/ |
//                                          |___/
//
// This file is auto-generated. Do not edit manually
//

use crate::app::gen::variations::gv::Variation;

#[derive(Debug, PartialEq)]
pub enum CountVariation {
    Group50Var1,
    Group50Var3,
    Group50Var4,
    Group51Var1,
    Group51Var2,
    Group52Var1,
    Group52Var2,
}

impl CountVariation {
    #[rustfmt::skip]
    pub fn get(v: Variation) -> Option<CountVariation> {
        match v {
            Variation::Group50Var1 => Some(CountVariation::Group50Var1),
            Variation::Group50Var3 => Some(CountVariation::Group50Var3),
            Variation::Group50Var4 => Some(CountVariation::Group50Var4),
            Variation::Group51Var1 => Some(CountVariation::Group51Var1),
            Variation::Group51Var2 => Some(CountVariation::Group51Var2),
            Variation::Group52Var1 => Some(CountVariation::Group52Var1),
            Variation::Group52Var2 => Some(CountVariation::Group52Var2),
            _ => None,
        }
    }
}
