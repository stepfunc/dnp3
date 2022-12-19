use dnp3::master::DeadBandHeader;

pub struct WriteDeadBandRequest {
    headers: Vec<Variant>,
    current: Option<Variant>,
}

enum Variant {
    /// Group 34 variation 1 with 8-bit index
    G34V1U8(Vec<(u16, u8)>),
    /// Group 34 variation 1 with 16-bit index
    G34V1U16(Vec<(u16, u16)>),
    /// Group 34 variation 2 with 8-bit index
    G34V2U8(Vec<(u32, u8)>),
    /// Group 34 variation 2 with 16-bit index
    G34V2U16(Vec<(u32, u16)>),
    /// Group 34 variation 3 with 8-bit index
    G34V3U8(Vec<(f32, u8)>),
    /// Group 34 variation 3 with 16-bit index
    G34V3U16(Vec<(f32, u16)>),
}

impl WriteDeadBandRequest {
    pub(crate) fn build(&mut self) -> Vec<dnp3::master::DeadBandHeader> {
        if let Some(x) = self.current.take() {
            self.headers.push(x);
        }

        let mut ret: Vec<DeadBandHeader> = Default::default();
        for header in self.headers.iter() {
            match header {
                Variant::G34V1U8(x) => {
                    ret.push(DeadBandHeader::group34_var1_u8(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Variant::G34V1U16(x) => {
                    ret.push(DeadBandHeader::group34_var1_u16(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Variant::G34V2U8(x) => {
                    ret.push(DeadBandHeader::group34_var2_u8(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Variant::G34V2U16(x) => {
                    ret.push(DeadBandHeader::group34_var2_u16(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Variant::G34V3U8(x) => {
                    ret.push(DeadBandHeader::group34_var3_u8(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Variant::G34V3U16(x) => {
                    ret.push(DeadBandHeader::group34_var3_u16(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
            }
        }

        ret
    }
}

pub(crate) fn write_dead_band_request_create() -> *mut crate::WriteDeadBandRequest {
    Box::into_raw(Box::new(WriteDeadBandRequest {
        headers: Default::default(),
        current: None,
    }))
}

pub(crate) unsafe fn write_dead_band_request_destroy(instance: *mut crate::WriteDeadBandRequest) {
    if !instance.is_null() {
        drop(Box::from_raw(instance));
    }
}
