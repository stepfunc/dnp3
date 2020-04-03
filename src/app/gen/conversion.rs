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

use crate::app::flags::Flags;
use crate::app::gen::variations::fixed::*;
use crate::app::meas::*;

impl std::convert::From<Group2Var2> for Binary {
    fn from(v: Group2Var2) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value: flags.state(),
            flags,
            time: Time::Synchronized(v.time),
        }
    }
}

impl std::convert::From<Group2Var1> for Binary {
    fn from(v: Group2Var1) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value: flags.state(),
            flags,
            time: Time::Invalid,
        }
    }
}

impl std::convert::From<Group1Var2> for Binary {
    fn from(v: Group1Var2) -> Self {
        let flags = Flags::new(v.flags);
        Binary {
            value: flags.state(),
            flags,
            time: Time::Invalid,
        }
    }
}
