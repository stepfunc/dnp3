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

use crate::app::gen::all::AllObjectsVariation;
use crate::app::parse::parser::ReadQualifier;
use crate::app::variations::Variation;

impl ReadQualifier {
    /// true if a READ request can carry the variation with this qualifier
    pub(crate) fn accepts(self, v: Variation) -> bool {
        match self {
            // a READ can name the same variations with all objects as any other function
            ReadQualifier::AllObjects => AllObjectsVariation::get(v).is_some(),
            ReadQualifier::Range(_, _) => matches!(
                v,
                Variation::Group0Var254
                | Variation::Group0(_)
                | Variation::Group1Var0
                | Variation::Group1Var1
                | Variation::Group1Var2
                | Variation::Group3Var0
                | Variation::Group3Var1
                | Variation::Group3Var2
                | Variation::Group10Var0
                | Variation::Group10Var1
                | Variation::Group10Var2
                | Variation::Group20Var0
                | Variation::Group20Var1
                | Variation::Group20Var2
                | Variation::Group20Var5
                | Variation::Group20Var6
                | Variation::Group21Var0
                | Variation::Group21Var1
                | Variation::Group21Var2
                | Variation::Group21Var5
                | Variation::Group21Var6
                | Variation::Group21Var9
                | Variation::Group21Var10
                | Variation::Group30Var0
                | Variation::Group30Var1
                | Variation::Group30Var2
                | Variation::Group30Var3
                | Variation::Group30Var4
                | Variation::Group30Var5
                | Variation::Group30Var6
                | Variation::Group31Var0
                | Variation::Group31Var1
                | Variation::Group31Var2
                | Variation::Group31Var3
                | Variation::Group31Var4
                | Variation::Group31Var5
                | Variation::Group31Var6
                | Variation::Group31Var7
                | Variation::Group31Var8
                | Variation::Group34Var0
                | Variation::Group34Var1
                | Variation::Group34Var2
                | Variation::Group34Var3
                | Variation::Group40Var0
                | Variation::Group40Var1
                | Variation::Group40Var2
                | Variation::Group40Var3
                | Variation::Group40Var4
                | Variation::Group80Var1
                | Variation::Group102Var0
                | Variation::Group102Var1
                | Variation::Group110(0)
            ),
            ReadQualifier::Count(_, _) => matches!(
                v,
                Variation::Group2Var0
                | Variation::Group2Var1
                | Variation::Group2Var2
                | Variation::Group2Var3
                | Variation::Group4Var0
                | Variation::Group4Var1
                | Variation::Group4Var2
                | Variation::Group4Var3
                | Variation::Group11Var0
                | Variation::Group11Var1
                | Variation::Group11Var2
                | Variation::Group13Var1
                | Variation::Group13Var2
                | Variation::Group22Var0
                | Variation::Group22Var1
                | Variation::Group22Var2
                | Variation::Group22Var5
                | Variation::Group22Var6
                | Variation::Group23Var0
                | Variation::Group23Var1
                | Variation::Group23Var2
                | Variation::Group23Var5
                | Variation::Group23Var6
                | Variation::Group32Var0
                | Variation::Group32Var1
                | Variation::Group32Var2
                | Variation::Group32Var3
                | Variation::Group32Var4
                | Variation::Group32Var5
                | Variation::Group32Var6
                | Variation::Group32Var7
                | Variation::Group32Var8
                | Variation::Group33Var0
                | Variation::Group33Var1
                | Variation::Group33Var2
                | Variation::Group33Var3
                | Variation::Group33Var4
                | Variation::Group33Var5
                | Variation::Group33Var6
                | Variation::Group33Var7
                | Variation::Group33Var8
                | Variation::Group42Var0
                | Variation::Group42Var1
                | Variation::Group42Var2
                | Variation::Group42Var3
                | Variation::Group42Var4
                | Variation::Group42Var5
                | Variation::Group42Var6
                | Variation::Group42Var7
                | Variation::Group42Var8
                | Variation::Group43Var1
                | Variation::Group43Var2
                | Variation::Group43Var3
                | Variation::Group43Var4
                | Variation::Group43Var5
                | Variation::Group43Var6
                | Variation::Group43Var7
                | Variation::Group43Var8
                | Variation::Group50Var1
                | Variation::Group50Var2
                | Variation::Group50Var3
                | Variation::Group50Var4
                | Variation::Group51Var1
                | Variation::Group51Var2
                | Variation::Group52Var1
                | Variation::Group52Var2
                | Variation::Group60Var2
                | Variation::Group60Var3
                | Variation::Group60Var4
                | Variation::Group111(_)
            ),
        }
    }
}
