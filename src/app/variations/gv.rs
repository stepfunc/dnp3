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

pub enum GroupVar {
    Group1Var0,
    Group1Var1,
    Group1Var2,
    Group2Var0,
    Group2Var1,
    Group2Var2,
    Group2Var3,
    Group3Var0,
    Group3Var1,
    Group3Var2,
    Group4Var0,
    Group4Var1,
    Group4Var2,
    Group4Var3,
    Group10Var0,
    Group10Var1,
    Group10Var2,
    Group11Var0,
    Group11Var1,
    Group11Var2,
    Group12Var0,
    Group12Var1,
    Group13Var1,
    Group13Var2,
    Group20Var0,
    Group20Var1,
    Group20Var2,
    Group20Var5,
    Group20Var6,
    Group21Var0,
    Group21Var1,
    Group21Var2,
    Group21Var5,
    Group21Var6,
    Group21Var9,
    Group21Var10,
    Group22Var0,
    Group22Var1,
    Group22Var2,
    Group22Var5,
    Group22Var6,
    Group23Var0,
    Group23Var1,
    Group23Var2,
    Group23Var5,
    Group23Var6,
    Group30Var0,
    Group30Var1,
    Group30Var2,
    Group30Var3,
    Group30Var4,
    Group30Var5,
    Group30Var6,
    Group32Var0,
    Group32Var1,
    Group32Var2,
    Group32Var3,
    Group32Var4,
    Group32Var5,
    Group32Var6,
    Group32Var7,
    Group32Var8,
    Group40Var0,
    Group40Var1,
    Group40Var2,
    Group40Var3,
    Group40Var4,
    Group41Var0,
    Group41Var1,
    Group41Var2,
    Group41Var3,
    Group41Var4,
    Group42Var0,
    Group42Var1,
    Group42Var2,
    Group42Var3,
    Group42Var4,
    Group42Var5,
    Group42Var6,
    Group42Var7,
    Group42Var8,
    Group43Var1,
    Group43Var2,
    Group43Var3,
    Group43Var4,
    Group43Var5,
    Group43Var6,
    Group43Var7,
    Group43Var8,
    Group50Var1,
    Group50Var3,
    Group50Var4,
    Group51Var1,
    Group51Var2,
    Group52Var1,
    Group52Var2,
    Group60Var1,
    Group60Var2,
    Group60Var3,
    Group60Var4,
}

impl GroupVar {
    pub fn lookup(group: u8, var: u8) -> Option<GroupVar> {
        match group {
            1 => match var {
                0 => Some(GroupVar::Group1Var0),
                1 => Some(GroupVar::Group1Var1),
                2 => Some(GroupVar::Group1Var2),
                _ => None,
            }
            2 => match var {
                0 => Some(GroupVar::Group2Var0),
                1 => Some(GroupVar::Group2Var1),
                2 => Some(GroupVar::Group2Var2),
                3 => Some(GroupVar::Group2Var3),
                _ => None,
            }
            3 => match var {
                0 => Some(GroupVar::Group3Var0),
                1 => Some(GroupVar::Group3Var1),
                2 => Some(GroupVar::Group3Var2),
                _ => None,
            }
            4 => match var {
                0 => Some(GroupVar::Group4Var0),
                1 => Some(GroupVar::Group4Var1),
                2 => Some(GroupVar::Group4Var2),
                3 => Some(GroupVar::Group4Var3),
                _ => None,
            }
            10 => match var {
                0 => Some(GroupVar::Group10Var0),
                1 => Some(GroupVar::Group10Var1),
                2 => Some(GroupVar::Group10Var2),
                _ => None,
            }
            11 => match var {
                0 => Some(GroupVar::Group11Var0),
                1 => Some(GroupVar::Group11Var1),
                2 => Some(GroupVar::Group11Var2),
                _ => None,
            }
            12 => match var {
                0 => Some(GroupVar::Group12Var0),
                1 => Some(GroupVar::Group12Var1),
                _ => None,
            }
            13 => match var {
                1 => Some(GroupVar::Group13Var1),
                2 => Some(GroupVar::Group13Var2),
                _ => None,
            }
            20 => match var {
                0 => Some(GroupVar::Group20Var0),
                1 => Some(GroupVar::Group20Var1),
                2 => Some(GroupVar::Group20Var2),
                5 => Some(GroupVar::Group20Var5),
                6 => Some(GroupVar::Group20Var6),
                _ => None,
            }
            21 => match var {
                0 => Some(GroupVar::Group21Var0),
                1 => Some(GroupVar::Group21Var1),
                2 => Some(GroupVar::Group21Var2),
                5 => Some(GroupVar::Group21Var5),
                6 => Some(GroupVar::Group21Var6),
                9 => Some(GroupVar::Group21Var9),
                10 => Some(GroupVar::Group21Var10),
                _ => None,
            }
            22 => match var {
                0 => Some(GroupVar::Group22Var0),
                1 => Some(GroupVar::Group22Var1),
                2 => Some(GroupVar::Group22Var2),
                5 => Some(GroupVar::Group22Var5),
                6 => Some(GroupVar::Group22Var6),
                _ => None,
            }
            23 => match var {
                0 => Some(GroupVar::Group23Var0),
                1 => Some(GroupVar::Group23Var1),
                2 => Some(GroupVar::Group23Var2),
                5 => Some(GroupVar::Group23Var5),
                6 => Some(GroupVar::Group23Var6),
                _ => None,
            }
            30 => match var {
                0 => Some(GroupVar::Group30Var0),
                1 => Some(GroupVar::Group30Var1),
                2 => Some(GroupVar::Group30Var2),
                3 => Some(GroupVar::Group30Var3),
                4 => Some(GroupVar::Group30Var4),
                5 => Some(GroupVar::Group30Var5),
                6 => Some(GroupVar::Group30Var6),
                _ => None,
            }
            32 => match var {
                0 => Some(GroupVar::Group32Var0),
                1 => Some(GroupVar::Group32Var1),
                2 => Some(GroupVar::Group32Var2),
                3 => Some(GroupVar::Group32Var3),
                4 => Some(GroupVar::Group32Var4),
                5 => Some(GroupVar::Group32Var5),
                6 => Some(GroupVar::Group32Var6),
                7 => Some(GroupVar::Group32Var7),
                8 => Some(GroupVar::Group32Var8),
                _ => None,
            }
            40 => match var {
                0 => Some(GroupVar::Group40Var0),
                1 => Some(GroupVar::Group40Var1),
                2 => Some(GroupVar::Group40Var2),
                3 => Some(GroupVar::Group40Var3),
                4 => Some(GroupVar::Group40Var4),
                _ => None,
            }
            41 => match var {
                0 => Some(GroupVar::Group41Var0),
                1 => Some(GroupVar::Group41Var1),
                2 => Some(GroupVar::Group41Var2),
                3 => Some(GroupVar::Group41Var3),
                4 => Some(GroupVar::Group41Var4),
                _ => None,
            }
            42 => match var {
                0 => Some(GroupVar::Group42Var0),
                1 => Some(GroupVar::Group42Var1),
                2 => Some(GroupVar::Group42Var2),
                3 => Some(GroupVar::Group42Var3),
                4 => Some(GroupVar::Group42Var4),
                5 => Some(GroupVar::Group42Var5),
                6 => Some(GroupVar::Group42Var6),
                7 => Some(GroupVar::Group42Var7),
                8 => Some(GroupVar::Group42Var8),
                _ => None,
            }
            43 => match var {
                1 => Some(GroupVar::Group43Var1),
                2 => Some(GroupVar::Group43Var2),
                3 => Some(GroupVar::Group43Var3),
                4 => Some(GroupVar::Group43Var4),
                5 => Some(GroupVar::Group43Var5),
                6 => Some(GroupVar::Group43Var6),
                7 => Some(GroupVar::Group43Var7),
                8 => Some(GroupVar::Group43Var8),
                _ => None,
            }
            50 => match var {
                1 => Some(GroupVar::Group50Var1),
                3 => Some(GroupVar::Group50Var3),
                4 => Some(GroupVar::Group50Var4),
                _ => None,
            }
            51 => match var {
                1 => Some(GroupVar::Group51Var1),
                2 => Some(GroupVar::Group51Var2),
                _ => None,
            }
            52 => match var {
                1 => Some(GroupVar::Group52Var1),
                2 => Some(GroupVar::Group52Var2),
                _ => None,
            }
            60 => match var {
                1 => Some(GroupVar::Group60Var1),
                2 => Some(GroupVar::Group60Var2),
                3 => Some(GroupVar::Group60Var3),
                4 => Some(GroupVar::Group60Var4),
                _ => None,
            }
            _ => None,
        }
    }
}
