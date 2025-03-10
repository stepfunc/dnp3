[1mdiff --git a/dnp3/src/app/attr.rs b/dnp3/src/app/attr.rs[m
[1mindex 4ec3a7f..6c6814a 100644[m
[1m--- a/dnp3/src/app/attr.rs[m
[1m+++ b/dnp3/src/app/attr.rs[m
[36m@@ -941,7 +941,7 @@[m [mpub enum AttrValue<'a> {[m
     AttrList(VariationList<'a>),[m
 }[m
 [m
[31m-impl<'a> AttrValue<'a> {[m
[32m+[m[32mimpl AttrValue<'_> {[m
     pub(crate) fn data_type(&self) -> AttrDataType {[m
         match self {[m
             Self::VisibleString(_) => AttrDataType::VisibleString,[m
[36m@@ -1301,7 +1301,7 @@[m [mpub struct Attribute<'a> {[m
     pub value: AttrValue<'a>,[m
 }[m
 [m
[31m-impl<'a> Attribute<'a> {[m
[32m+[m[32mimpl Attribute<'_> {[m
     pub(crate) fn to_owned(self) -> Option<OwnedAttribute> {[m
         let value = self.value.to_owned()?;[m
         Some(OwnedAttribute {[m
[1mdiff --git a/dnp3/src/app/file/g70v5.rs b/dnp3/src/app/file/g70v5.rs[m
[1mindex 8f639ac..e1df059 100644[m
[1m--- a/dnp3/src/app/file/g70v5.rs[m
[1m+++ b/dnp3/src/app/file/g70v5.rs[m
[36m@@ -47,7 +47,7 @@[m [mmod test {[m
     const OBJECT: Group70Var5 = Group70Var5 {[m
         file_handle: 0x01020304,[m
         block_number: 0xFECAADDE,[m
[31m-        file_data: &[b'd', b'a', b't', b'a'],[m
[32m+[m[32m        file_data: b"data",[m
     };[m
 [m
     const DATA: &[u8] = &[[m
[1mdiff --git a/dnp3/src/app/file/g70v8.rs b/dnp3/src/app/file/g70v8.rs[m
[1mindex 5d336b1..79b303e 100644[m
[1m--- a/dnp3/src/app/file/g70v8.rs[m
[1m+++ b/dnp3/src/app/file/g70v8.rs[m
[36m@@ -38,7 +38,7 @@[m [mmod test {[m
         file_specification: "test",[m
     };[m
 [m
[31m-    const DATA: &[u8] = &[b't', b'e', b's', b't'];[m
[32m+[m[32m    const DATA: &[u8] = b"test";[m
 [m
     #[test][m
     fn writes_valid_object() {[m
[1mdiff --git a/dnp3/src/app/format/write.rs b/dnp3/src/app/format/write.rs[m
[1mindex 7a3607a..d6ab8ff 100644[m
[1m--- a/dnp3/src/app/format/write.rs[m
[1m+++ b/dnp3/src/app/format/write.rs[m
[36m@@ -236,7 +236,7 @@[m [mmod test {[m
         let data = Group70Var5 {[m
             file_handle: 0xFFEEDDCC,[m
             block_number: 0x01ABCDEF,[m
[31m-            file_data: &[b'h', b'i'],[m
[32m+[m[32m            file_data: b"hi",[m
         };[m
 [m
         writer.write_free_format(&data).unwrap();[m
[1mdiff --git a/dnp3/src/app/parse/parser.rs b/dnp3/src/app/parse/parser.rs[m
[1mindex ed1841d..68c47a1 100644[m
[1m--- a/dnp3/src/app/parse/parser.rs[m
[1m+++ b/dnp3/src/app/parse/parser.rs[m
[36m@@ -1219,7 +1219,7 @@[m [mmod test {[m
             crate::app::file::Group70Var5 {[m
                 file_handle: 0x04030201,[m
                 block_number: 0xDDCCBBAA,[m
[31m-                file_data: &[b'd', b'a', b't', b'a'],[m
[32m+[m[32m                file_data: b"data",[m
             }[m
         );[m
     }[m
[1mdiff --git a/dnp3/src/master/tests/file/mod.rs b/dnp3/src/master/tests/file/mod.rs[m
[1mindex eb8f9b3..1015e77 100644[m
[1m--- a/dnp3/src/master/tests/file/mod.rs[m
[1m+++ b/dnp3/src/master/tests/file/mod.rs[m
[36m@@ -8,7 +8,7 @@[m [mmod close_file;[m
 mod open_file;[m
 mod read_file;[m
 [m
[31m-impl<'a> FreeFormat for Group70Var6<'a> {[m
[32m+[m[32mimpl FreeFormat for Group70Var6<'_> {[m
     const VARIATION: Variation = Variation::Group70Var6;[m
 [m
     fn write(&self, cursor: &mut WriteCursor) -> Result<(), WriteError> {[m
[36m@@ -17,7 +17,7 @@[m [mimpl<'a> FreeFormat for Group70Var6<'a> {[m
 }[m
 [m
 pub(super) fn last_block(block: u32) -> u32 {[m
[31m-    1 << 31 | block[m
[32m+[m[32m    (1 << 31) | block[m
 }[m
 [m
 pub(super) fn fir_and_fin(seq: u8) -> u8 {[m
[1mdiff --git a/dnp3/src/outstation/control/collection.rs b/dnp3/src/outstation/control/collection.rs[m
[1mindex 3b9151e..952b9e2 100644[m
[1m--- a/dnp3/src/outstation/control/collection.rs[m
[1m+++ b/dnp3/src/outstation/control/collection.rs[m
[36m@@ -48,7 +48,7 @@[m [mimpl<'a> ControlTransaction<'a> {[m
     }[m
 }[m
 [m
[31m-impl<'a> ControlSupport<Group12Var1> for ControlTransaction<'a> {[m
[32m+[m[32mimpl ControlSupport<Group12Var1> for ControlTransaction<'_> {[m
     fn select([m
         &mut self,[m
         control: Group12Var1,[m
[36m@@ -71,7 +71,7 @@[m [mimpl<'a> ControlSupport<Group12Var1> for ControlTransaction<'a> {[m
     }[m
 }[m
 [m
[31m-impl<'a> ControlSupport<Group41Var1> for ControlTransaction<'a> {[m
[32m+[m[32mimpl ControlSupport<Group41Var1> for ControlTransaction<'_> {[m
     fn select([m
         &mut self,[m
         control: Group41Var1,[m
[36m@@ -94,7 +94,7 @@[m [mimpl<'a> ControlSupport<Group41Var1> for ControlTransaction<'a> {[m
     }[m
 }[m
 [m
[31m-impl<'a> ControlSupport<Group41Var2> for ControlTransaction<'a> {[m
[32m+[m[32mimpl ControlSupport<Group41Var2> for ControlTransaction<'_> {[m
     fn select([m
         &mut self,[m
         control: Group41Var2,[m
[36m@@ -117,7 +117,7 @@[m [mimpl<'a> ControlSupport<Group41Var2> for ControlTransaction<'a> {[m
     }[m
 }[m
 [m
[31m-impl<'a> ControlSupport<Group41Var3> for ControlTransaction<'a> {[m
[32m+[m[32mimpl ControlSupport<Group41Var3> for ControlTransaction<'_> {[m
     fn select([m
         &mut self,[m
         control: Group41Var3,[m
[36m@@ -140,7 +140,7 @@[m [mimpl<'a> ControlSupport<Group41Var3> for ControlTransaction<'a> {[m
     }[m
 }[m
 [m
[31m-impl<'a> ControlSupport<Group41Var4> for ControlTransaction<'a> {[m
[32m+[m[32mimpl ControlSupport<Group41Var4> for ControlTransaction<'_> {[m
     fn select([m
         &mut self,[m
         control: Group41Var4,[m
[36m@@ -359,7 +359,7 @@[m [mimpl<'a> Iterator for ControlHeaderIterator<'a> {[m
     }[m
 }[m
 [m
[31m-impl<'a> ControlHeader<'a> {[m
[32m+[m[32mimpl ControlHeader<'_> {[m
     fn respond_with_status([m
         &self,[m
         cursor: &mut WriteCursor,[m
