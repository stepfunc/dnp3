use dnp3::master::DeadBandHeader;

pub struct WriteDeadBandRequest {
    headers: Vec<Header>,
    current: Option<Header>,
}

enum Header {
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
                Header::G34V1U8(x) => {
                    ret.push(DeadBandHeader::group34_var1_u8(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Header::G34V1U16(x) => {
                    ret.push(DeadBandHeader::group34_var1_u16(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Header::G34V2U8(x) => {
                    ret.push(DeadBandHeader::group34_var2_u8(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Header::G34V2U16(x) => {
                    ret.push(DeadBandHeader::group34_var2_u16(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Header::G34V3U8(x) => {
                    ret.push(DeadBandHeader::group34_var3_u8(
                        x.iter().map(|(i, v)| (*v, *i)).collect(),
                    ));
                }
                Header::G34V3U16(x) => {
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

pub(crate) unsafe fn write_dead_band_request_add_g34v1_u8(
    instance: *mut crate::WriteDeadBandRequest,
    index: u8,
    dead_band: u16,
) {
    write_dead_band_request_add_generic(instance, (dead_band, index))
}

pub(crate) unsafe fn write_dead_band_request_add_g34v1_u16(
    instance: *mut crate::WriteDeadBandRequest,
    index: u16,
    dead_band: u16,
) {
    write_dead_band_request_add_generic(instance, (dead_band, index))
}

pub(crate) unsafe fn write_dead_band_request_add_g34v2_u8(
    instance: *mut crate::WriteDeadBandRequest,
    index: u8,
    dead_band: u32,
) {
    write_dead_band_request_add_generic(instance, (dead_band, index))
}

pub(crate) unsafe fn write_dead_band_request_add_g34v2_u16(
    instance: *mut crate::WriteDeadBandRequest,
    index: u16,
    dead_band: u32,
) {
    write_dead_band_request_add_generic(instance, (dead_band, index))
}

pub(crate) unsafe fn write_dead_band_request_add_g34v3_u8(
    instance: *mut crate::WriteDeadBandRequest,
    index: u8,
    dead_band: f32,
) {
    write_dead_band_request_add_generic(instance, (dead_band, index))
}

pub(crate) unsafe fn write_dead_band_request_add_g34v3_u16(
    instance: *mut crate::WriteDeadBandRequest,
    index: u16,
    dead_band: f32,
) {
    write_dead_band_request_add_generic(instance, (dead_band, index))
}

pub(crate) unsafe fn write_dead_band_request_finish_header(
    instance: *mut crate::WriteDeadBandRequest,
) {
    if let Some(instance) = instance.as_mut() {
        if let Some(current) = instance.current.take() {
            instance.headers.push(current);
        }
    }
}

unsafe fn write_dead_band_request_add_generic<T>(
    instance: *mut crate::WriteDeadBandRequest,
    item: T,
) where
    T: DeadBandVariant,
{
    let instance = match instance.as_mut() {
        None => return,
        Some(x) => x,
    };

    match instance.current.take() {
        None => instance.current = Some(T::wrap(item)),
        Some(v) => match T::extend(v, item) {
            Ok(x) => instance.current = Some(x),
            Err((old, new)) => {
                instance.headers.push(old);
                instance.current = Some(new);
            }
        },
    }
}

trait DeadBandVariant {
    fn wrap(item: Self) -> Header;
    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)>;
}

impl DeadBandVariant for (u16, u8) {
    fn wrap(item: Self) -> Header {
        Header::G34V1U8(vec![item])
    }

    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)> {
        match header {
            Header::G34V1U8(mut x) => {
                x.push(item);
                Ok(Header::G34V1U8(x))
            }
            _ => Err((header, Self::wrap(item))),
        }
    }
}

impl DeadBandVariant for (u16, u16) {
    fn wrap(item: Self) -> Header {
        Header::G34V1U16(vec![item])
    }

    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)> {
        match header {
            Header::G34V1U16(mut x) => {
                x.push(item);
                Ok(Header::G34V1U16(x))
            }
            _ => Err((header, Self::wrap(item))),
        }
    }
}

impl DeadBandVariant for (u32, u8) {
    fn wrap(item: Self) -> Header {
        Header::G34V2U8(vec![item])
    }

    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)> {
        match header {
            Header::G34V2U8(mut x) => {
                x.push(item);
                Ok(Header::G34V2U8(x))
            }
            _ => Err((header, Self::wrap(item))),
        }
    }
}

impl DeadBandVariant for (u32, u16) {
    fn wrap(item: Self) -> Header {
        Header::G34V2U16(vec![item])
    }

    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)> {
        match header {
            Header::G34V2U16(mut x) => {
                x.push(item);
                Ok(Header::G34V2U16(x))
            }
            _ => Err((header, Self::wrap(item))),
        }
    }
}

impl DeadBandVariant for (f32, u8) {
    fn wrap(item: Self) -> Header {
        Header::G34V3U8(vec![item])
    }

    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)> {
        match header {
            Header::G34V3U8(mut x) => {
                x.push(item);
                Ok(Header::G34V3U8(x))
            }
            _ => Err((header, Self::wrap(item))),
        }
    }
}

impl DeadBandVariant for (f32, u16) {
    fn wrap(item: Self) -> Header {
        Header::G34V3U16(vec![item])
    }

    fn extend(header: Header, item: Self) -> Result<Header, (Header, Header)> {
        match header {
            Header::G34V3U16(mut x) => {
                x.push(item);
                Ok(Header::G34V3U16(x))
            }
            _ => Err((header, Self::wrap(item))),
        }
    }
}
