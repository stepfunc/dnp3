use dnp3::app::attr::*;
use dnp3::app::{Timestamp, Variation};
use dnp3::master::{Headers, ReadHeader, ReadRequest};
use dnp3::outstation::FreezeInterval;
use std::ffi::CStr;

use crate::ffi;

pub struct Request {
    headers: Headers,
}

impl Request {
    fn new() -> Self {
        Self {
            headers: Default::default(),
        }
    }

    pub(crate) fn add_read_header(&mut self, header: ReadHeader) {
        self.headers.push_read_header(header);
    }

    pub(crate) fn add_attribute(&mut self, attr: OwnedAttribute) {
        self.headers.push_attr(attr);
    }

    pub(crate) fn add_time_and_interval(&mut self, time: Timestamp, interval: u32) {
        self.headers
            .push_freeze_interval(FreezeInterval::new(time, interval));
    }

    pub(crate) fn build_read_request(&self) -> ReadRequest {
        self.headers.to_read_request()
    }

    pub(crate) fn build_headers(&self) -> Headers {
        self.headers.clone()
    }
}

pub unsafe fn request_create() -> *mut Request {
    let request = Box::new(Request::new());
    Box::into_raw(request)
}

pub unsafe fn request_new_class(
    class0: bool,
    class1: bool,
    class2: bool,
    class3: bool,
) -> *mut Request {
    let mut request = Request::new();
    if class1 {
        request.add_read_header(ReadHeader::all_objects(Variation::Group60Var2));
    }
    if class2 {
        request.add_read_header(ReadHeader::all_objects(Variation::Group60Var3));
    }
    if class3 {
        request.add_read_header(ReadHeader::all_objects(Variation::Group60Var4));
    }
    if class0 {
        request.add_read_header(ReadHeader::all_objects(Variation::Group60Var1));
    }

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_all_objects(variation: ffi::Variation) -> *mut Request {
    let mut request = Request::new();
    request.add_read_header(ReadHeader::all_objects(variation.into()));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_one_byte_range(
    variation: ffi::Variation,
    start: u8,
    stop: u8,
) -> *mut Request {
    let mut request = Request::new();
    request.add_read_header(ReadHeader::one_byte_range(variation.into(), start, stop));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_two_byte_range(
    variation: ffi::Variation,
    start: u16,
    stop: u16,
) -> *mut Request {
    let mut request = Request::new();
    request.add_read_header(ReadHeader::two_byte_range(variation.into(), start, stop));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_one_byte_limited_count(
    variation: ffi::Variation,
    count: u8,
) -> *mut Request {
    let mut request = Request::new();
    request.add_read_header(ReadHeader::one_byte_limited_count(variation.into(), count));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_two_byte_limited_count(
    variation: ffi::Variation,
    count: u16,
) -> *mut Request {
    let mut request = Request::new();
    request.add_read_header(ReadHeader::two_byte_limited_count(variation.into(), count));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_destroy(request: *mut Request) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

pub(crate) unsafe fn request_add_one_byte_range_header(
    request: *mut Request,
    variation: ffi::Variation,
    start: u8,
    stop: u8,
) {
    if let Some(request) = request.as_mut() {
        request.add_read_header(ReadHeader::one_byte_range(variation.into(), start, stop));
    }
}

pub(crate) unsafe fn request_add_specific_attribute(
    request: *mut crate::Request,
    variation: u8,
    set: u8,
) {
    if let Some(request) = request.as_mut() {
        request.add_read_header(ReadHeader::one_byte_range(
            Variation::Group0(variation),
            set,
            set,
        ));
    }
}

pub(crate) unsafe fn request_add_string_attribute(
    request: *mut crate::Request,
    variation: u8,
    set: u8,
    value: &CStr,
) {
    if let Some(request) = request.as_mut() {
        request.add_attribute(OwnedAttribute::new(
            AttrSet::new(set),
            variation,
            OwnedAttrValue::VisibleString(value.to_string_lossy().to_string()),
        ))
    }
}

pub(crate) unsafe fn request_add_uint_attribute(
    request: *mut crate::Request,
    variation: u8,
    set: u8,
    value: u32,
) {
    if let Some(request) = request.as_mut() {
        request.add_attribute(OwnedAttribute::new(
            AttrSet::new(set),
            variation,
            OwnedAttrValue::UnsignedInt(value),
        ))
    }
}

pub(crate) unsafe fn request_add_two_byte_range_header(
    request: *mut Request,
    variation: ffi::Variation,
    start: u16,
    stop: u16,
) {
    if let Some(request) = request.as_mut() {
        request.add_read_header(ReadHeader::two_byte_range(variation.into(), start, stop));
    }
}

pub(crate) unsafe fn request_add_all_objects_header(
    request: *mut Request,
    variation: ffi::Variation,
) {
    if let Some(request) = request.as_mut() {
        request.add_read_header(ReadHeader::all_objects(variation.into()));
    }
}

pub unsafe fn request_add_one_byte_limited_count_header(
    request: *mut Request,
    variation: ffi::Variation,
    count: u8,
) {
    if let Some(request) = request.as_mut() {
        request.add_read_header(ReadHeader::one_byte_limited_count(variation.into(), count));
    }
}

pub unsafe fn request_add_two_byte_limited_count_header(
    request: *mut Request,
    variation: ffi::Variation,
    count: u16,
) {
    if let Some(request) = request.as_mut() {
        request.add_read_header(ReadHeader::two_byte_limited_count(variation.into(), count));
    }
}

pub(crate) unsafe fn request_add_time_and_interval(
    request: *mut crate::Request,
    time: u64,
    interval_ms: u32,
) {
    if let Some(request) = request.as_mut() {
        request.add_time_and_interval(Timestamp::new(time), interval_ms);
    }
}

impl From<ffi::Variation> for Variation {
    fn from(from: ffi::Variation) -> Variation {
        match from {
            ffi::Variation::Group0 => Variation::Group0(0),
            ffi::Variation::Group0Var254 => Variation::Group0Var254,
            ffi::Variation::Group1Var0 => Variation::Group1Var0,
            ffi::Variation::Group1Var1 => Variation::Group1Var1,
            ffi::Variation::Group1Var2 => Variation::Group1Var2,
            ffi::Variation::Group2Var0 => Variation::Group2Var0,
            ffi::Variation::Group2Var1 => Variation::Group2Var1,
            ffi::Variation::Group2Var2 => Variation::Group2Var2,
            ffi::Variation::Group2Var3 => Variation::Group2Var3,
            ffi::Variation::Group3Var0 => Variation::Group3Var0,
            ffi::Variation::Group3Var1 => Variation::Group3Var1,
            ffi::Variation::Group3Var2 => Variation::Group3Var2,
            ffi::Variation::Group4Var0 => Variation::Group4Var0,
            ffi::Variation::Group4Var1 => Variation::Group4Var1,
            ffi::Variation::Group4Var2 => Variation::Group4Var2,
            ffi::Variation::Group4Var3 => Variation::Group4Var3,
            ffi::Variation::Group10Var0 => Variation::Group10Var0,
            ffi::Variation::Group10Var1 => Variation::Group10Var1,
            ffi::Variation::Group10Var2 => Variation::Group10Var2,
            ffi::Variation::Group11Var0 => Variation::Group11Var0,
            ffi::Variation::Group11Var1 => Variation::Group11Var1,
            ffi::Variation::Group11Var2 => Variation::Group11Var2,
            ffi::Variation::Group12Var1 => Variation::Group12Var1,
            ffi::Variation::Group13Var1 => Variation::Group13Var1,
            ffi::Variation::Group13Var2 => Variation::Group13Var2,
            ffi::Variation::Group20Var0 => Variation::Group20Var0,
            ffi::Variation::Group20Var1 => Variation::Group20Var1,
            ffi::Variation::Group20Var2 => Variation::Group20Var2,
            ffi::Variation::Group20Var5 => Variation::Group20Var5,
            ffi::Variation::Group20Var6 => Variation::Group20Var6,
            ffi::Variation::Group21Var0 => Variation::Group21Var0,
            ffi::Variation::Group21Var1 => Variation::Group21Var1,
            ffi::Variation::Group21Var2 => Variation::Group21Var2,
            ffi::Variation::Group21Var5 => Variation::Group21Var5,
            ffi::Variation::Group21Var6 => Variation::Group21Var6,
            ffi::Variation::Group21Var9 => Variation::Group21Var9,
            ffi::Variation::Group21Var10 => Variation::Group21Var10,
            ffi::Variation::Group22Var0 => Variation::Group22Var0,
            ffi::Variation::Group22Var1 => Variation::Group22Var1,
            ffi::Variation::Group22Var2 => Variation::Group22Var2,
            ffi::Variation::Group22Var5 => Variation::Group22Var5,
            ffi::Variation::Group22Var6 => Variation::Group22Var6,
            ffi::Variation::Group23Var0 => Variation::Group23Var0,
            ffi::Variation::Group23Var1 => Variation::Group23Var1,
            ffi::Variation::Group23Var2 => Variation::Group23Var2,
            ffi::Variation::Group23Var5 => Variation::Group23Var5,
            ffi::Variation::Group23Var6 => Variation::Group23Var6,
            ffi::Variation::Group30Var0 => Variation::Group30Var0,
            ffi::Variation::Group30Var1 => Variation::Group30Var1,
            ffi::Variation::Group30Var2 => Variation::Group30Var2,
            ffi::Variation::Group30Var3 => Variation::Group30Var3,
            ffi::Variation::Group30Var4 => Variation::Group30Var4,
            ffi::Variation::Group30Var5 => Variation::Group30Var5,
            ffi::Variation::Group30Var6 => Variation::Group30Var6,
            ffi::Variation::Group31Var0 => Variation::Group31Var0,
            ffi::Variation::Group31Var1 => Variation::Group31Var1,
            ffi::Variation::Group31Var2 => Variation::Group31Var2,
            ffi::Variation::Group31Var3 => Variation::Group31Var3,
            ffi::Variation::Group31Var4 => Variation::Group31Var4,
            ffi::Variation::Group31Var5 => Variation::Group31Var5,
            ffi::Variation::Group31Var6 => Variation::Group31Var6,
            ffi::Variation::Group31Var7 => Variation::Group31Var7,
            ffi::Variation::Group31Var8 => Variation::Group31Var8,
            ffi::Variation::Group32Var0 => Variation::Group32Var0,
            ffi::Variation::Group32Var1 => Variation::Group32Var1,
            ffi::Variation::Group32Var2 => Variation::Group32Var2,
            ffi::Variation::Group32Var3 => Variation::Group32Var3,
            ffi::Variation::Group32Var4 => Variation::Group32Var4,
            ffi::Variation::Group32Var5 => Variation::Group32Var5,
            ffi::Variation::Group32Var6 => Variation::Group32Var6,
            ffi::Variation::Group32Var7 => Variation::Group32Var7,
            ffi::Variation::Group32Var8 => Variation::Group32Var8,

            ffi::Variation::Group33Var0 => Variation::Group33Var0,
            ffi::Variation::Group33Var1 => Variation::Group33Var1,
            ffi::Variation::Group33Var2 => Variation::Group33Var2,
            ffi::Variation::Group33Var3 => Variation::Group33Var3,
            ffi::Variation::Group33Var4 => Variation::Group33Var4,
            ffi::Variation::Group33Var5 => Variation::Group33Var5,
            ffi::Variation::Group33Var6 => Variation::Group33Var6,
            ffi::Variation::Group33Var7 => Variation::Group33Var7,
            ffi::Variation::Group33Var8 => Variation::Group33Var8,

            ffi::Variation::Group34Var0 => Variation::Group34Var0,
            ffi::Variation::Group34Var1 => Variation::Group34Var1,
            ffi::Variation::Group34Var2 => Variation::Group34Var2,
            ffi::Variation::Group34Var3 => Variation::Group34Var3,

            ffi::Variation::Group40Var0 => Variation::Group40Var0,
            ffi::Variation::Group40Var1 => Variation::Group40Var1,
            ffi::Variation::Group40Var2 => Variation::Group40Var2,
            ffi::Variation::Group40Var3 => Variation::Group40Var3,
            ffi::Variation::Group40Var4 => Variation::Group40Var4,
            ffi::Variation::Group41Var1 => Variation::Group41Var1,
            ffi::Variation::Group41Var2 => Variation::Group41Var2,
            ffi::Variation::Group41Var3 => Variation::Group41Var3,
            ffi::Variation::Group41Var4 => Variation::Group41Var4,
            ffi::Variation::Group42Var0 => Variation::Group42Var0,
            ffi::Variation::Group42Var1 => Variation::Group42Var1,
            ffi::Variation::Group42Var2 => Variation::Group42Var2,
            ffi::Variation::Group42Var3 => Variation::Group42Var3,
            ffi::Variation::Group42Var4 => Variation::Group42Var4,
            ffi::Variation::Group42Var5 => Variation::Group42Var5,
            ffi::Variation::Group42Var6 => Variation::Group42Var6,
            ffi::Variation::Group42Var7 => Variation::Group42Var7,
            ffi::Variation::Group42Var8 => Variation::Group42Var8,
            ffi::Variation::Group43Var1 => Variation::Group43Var1,
            ffi::Variation::Group43Var2 => Variation::Group43Var2,
            ffi::Variation::Group43Var3 => Variation::Group43Var3,
            ffi::Variation::Group43Var4 => Variation::Group43Var4,
            ffi::Variation::Group43Var5 => Variation::Group43Var5,
            ffi::Variation::Group43Var6 => Variation::Group43Var6,
            ffi::Variation::Group43Var7 => Variation::Group43Var7,
            ffi::Variation::Group43Var8 => Variation::Group43Var8,
            ffi::Variation::Group50Var1 => Variation::Group50Var1,
            ffi::Variation::Group50Var2 => Variation::Group50Var2,
            ffi::Variation::Group50Var3 => Variation::Group50Var3,
            ffi::Variation::Group50Var4 => Variation::Group50Var4,
            ffi::Variation::Group51Var1 => Variation::Group51Var1,
            ffi::Variation::Group51Var2 => Variation::Group51Var2,
            ffi::Variation::Group52Var1 => Variation::Group52Var1,
            ffi::Variation::Group52Var2 => Variation::Group52Var2,
            ffi::Variation::Group60Var1 => Variation::Group60Var1,
            ffi::Variation::Group60Var2 => Variation::Group60Var2,
            ffi::Variation::Group60Var3 => Variation::Group60Var3,
            ffi::Variation::Group60Var4 => Variation::Group60Var4,
            // group 70
            ffi::Variation::Group70Var2 => Variation::Group70Var2,
            ffi::Variation::Group70Var3 => Variation::Group70Var3,
            ffi::Variation::Group70Var4 => Variation::Group70Var4,
            ffi::Variation::Group70Var5 => Variation::Group70Var5,
            ffi::Variation::Group70Var6 => Variation::Group70Var6,
            ffi::Variation::Group70Var7 => Variation::Group70Var7,
            ffi::Variation::Group70Var8 => Variation::Group70Var8,

            ffi::Variation::Group80Var1 => Variation::Group80Var1,

            ffi::Variation::Group102Var0 => Variation::Group102Var0,
            ffi::Variation::Group102Var1 => Variation::Group102Var1,

            ffi::Variation::Group110 => Variation::Group110(0),
            ffi::Variation::Group111 => Variation::Group111(0),
            /*
            ffi::Variation::Group112 => Variation::Group112(0),
            ffi::Variation::Group113 => Variation::Group113(0),
             */
        }
    }
}

impl From<Variation> for ffi::Variation {
    fn from(from: Variation) -> ffi::Variation {
        match from {
            Variation::Group0(_) => ffi::Variation::Group0,
            Variation::Group0Var254 => ffi::Variation::Group0Var254,
            Variation::Group1Var0 => ffi::Variation::Group1Var0,
            Variation::Group1Var1 => ffi::Variation::Group1Var1,
            Variation::Group1Var2 => ffi::Variation::Group1Var2,
            Variation::Group2Var0 => ffi::Variation::Group2Var0,
            Variation::Group2Var1 => ffi::Variation::Group2Var1,
            Variation::Group2Var2 => ffi::Variation::Group2Var2,
            Variation::Group2Var3 => ffi::Variation::Group2Var3,
            Variation::Group3Var0 => ffi::Variation::Group3Var0,
            Variation::Group3Var1 => ffi::Variation::Group3Var1,
            Variation::Group3Var2 => ffi::Variation::Group3Var2,
            Variation::Group4Var0 => ffi::Variation::Group4Var0,
            Variation::Group4Var1 => ffi::Variation::Group4Var1,
            Variation::Group4Var2 => ffi::Variation::Group4Var2,
            Variation::Group4Var3 => ffi::Variation::Group4Var3,
            Variation::Group10Var0 => ffi::Variation::Group10Var0,
            Variation::Group10Var1 => ffi::Variation::Group10Var1,
            Variation::Group10Var2 => ffi::Variation::Group10Var2,
            Variation::Group11Var0 => ffi::Variation::Group11Var0,
            Variation::Group11Var1 => ffi::Variation::Group11Var1,
            Variation::Group11Var2 => ffi::Variation::Group11Var2,
            Variation::Group12Var1 => ffi::Variation::Group12Var1,
            Variation::Group13Var1 => ffi::Variation::Group13Var1,
            Variation::Group13Var2 => ffi::Variation::Group13Var2,
            Variation::Group20Var0 => ffi::Variation::Group20Var0,
            Variation::Group20Var1 => ffi::Variation::Group20Var1,
            Variation::Group20Var2 => ffi::Variation::Group20Var2,
            Variation::Group20Var5 => ffi::Variation::Group20Var5,
            Variation::Group20Var6 => ffi::Variation::Group20Var6,
            Variation::Group21Var0 => ffi::Variation::Group21Var0,
            Variation::Group21Var1 => ffi::Variation::Group21Var1,
            Variation::Group21Var2 => ffi::Variation::Group21Var2,
            Variation::Group21Var5 => ffi::Variation::Group21Var5,
            Variation::Group21Var6 => ffi::Variation::Group21Var6,
            Variation::Group21Var9 => ffi::Variation::Group21Var9,
            Variation::Group21Var10 => ffi::Variation::Group21Var10,
            Variation::Group22Var0 => ffi::Variation::Group22Var0,
            Variation::Group22Var1 => ffi::Variation::Group22Var1,
            Variation::Group22Var2 => ffi::Variation::Group22Var2,
            Variation::Group22Var5 => ffi::Variation::Group22Var5,
            Variation::Group22Var6 => ffi::Variation::Group22Var6,
            Variation::Group23Var0 => ffi::Variation::Group23Var0,
            Variation::Group23Var1 => ffi::Variation::Group23Var1,
            Variation::Group23Var2 => ffi::Variation::Group23Var2,
            Variation::Group23Var5 => ffi::Variation::Group23Var5,
            Variation::Group23Var6 => ffi::Variation::Group23Var6,
            Variation::Group30Var0 => ffi::Variation::Group30Var0,
            Variation::Group30Var1 => ffi::Variation::Group30Var1,
            Variation::Group30Var2 => ffi::Variation::Group30Var2,
            Variation::Group30Var3 => ffi::Variation::Group30Var3,
            Variation::Group30Var4 => ffi::Variation::Group30Var4,
            Variation::Group30Var5 => ffi::Variation::Group30Var5,
            Variation::Group30Var6 => ffi::Variation::Group30Var6,

            Variation::Group31Var0 => ffi::Variation::Group31Var0,
            Variation::Group31Var1 => ffi::Variation::Group31Var1,
            Variation::Group31Var2 => ffi::Variation::Group31Var2,
            Variation::Group31Var3 => ffi::Variation::Group31Var3,
            Variation::Group31Var4 => ffi::Variation::Group31Var4,
            Variation::Group31Var5 => ffi::Variation::Group31Var5,
            Variation::Group31Var6 => ffi::Variation::Group31Var6,
            Variation::Group31Var7 => ffi::Variation::Group31Var7,
            Variation::Group31Var8 => ffi::Variation::Group31Var8,

            Variation::Group32Var0 => ffi::Variation::Group32Var0,
            Variation::Group32Var1 => ffi::Variation::Group32Var1,
            Variation::Group32Var2 => ffi::Variation::Group32Var2,
            Variation::Group32Var3 => ffi::Variation::Group32Var3,
            Variation::Group32Var4 => ffi::Variation::Group32Var4,
            Variation::Group32Var5 => ffi::Variation::Group32Var5,
            Variation::Group32Var6 => ffi::Variation::Group32Var6,
            Variation::Group32Var7 => ffi::Variation::Group32Var7,
            Variation::Group32Var8 => ffi::Variation::Group32Var8,

            Variation::Group33Var0 => ffi::Variation::Group33Var0,
            Variation::Group33Var1 => ffi::Variation::Group33Var1,
            Variation::Group33Var2 => ffi::Variation::Group33Var2,
            Variation::Group33Var3 => ffi::Variation::Group33Var3,
            Variation::Group33Var4 => ffi::Variation::Group33Var4,
            Variation::Group33Var5 => ffi::Variation::Group33Var5,
            Variation::Group33Var6 => ffi::Variation::Group33Var6,
            Variation::Group33Var7 => ffi::Variation::Group33Var7,
            Variation::Group33Var8 => ffi::Variation::Group33Var8,

            Variation::Group34Var0 => ffi::Variation::Group34Var0,
            Variation::Group34Var1 => ffi::Variation::Group34Var1,
            Variation::Group34Var2 => ffi::Variation::Group34Var2,
            Variation::Group34Var3 => ffi::Variation::Group34Var3,

            Variation::Group40Var0 => ffi::Variation::Group40Var0,
            Variation::Group40Var1 => ffi::Variation::Group40Var1,
            Variation::Group40Var2 => ffi::Variation::Group40Var2,
            Variation::Group40Var3 => ffi::Variation::Group40Var3,
            Variation::Group40Var4 => ffi::Variation::Group40Var4,

            Variation::Group43Var1 => ffi::Variation::Group43Var1,
            Variation::Group43Var2 => ffi::Variation::Group43Var2,
            Variation::Group43Var3 => ffi::Variation::Group43Var3,
            Variation::Group43Var4 => ffi::Variation::Group43Var4,
            Variation::Group43Var5 => ffi::Variation::Group43Var5,
            Variation::Group43Var6 => ffi::Variation::Group43Var6,
            Variation::Group43Var7 => ffi::Variation::Group43Var7,
            Variation::Group43Var8 => ffi::Variation::Group43Var8,

            Variation::Group41Var1 => ffi::Variation::Group41Var1,
            Variation::Group41Var2 => ffi::Variation::Group41Var2,
            Variation::Group41Var3 => ffi::Variation::Group41Var3,
            Variation::Group41Var4 => ffi::Variation::Group41Var4,
            Variation::Group42Var0 => ffi::Variation::Group42Var0,
            Variation::Group42Var1 => ffi::Variation::Group42Var1,
            Variation::Group42Var2 => ffi::Variation::Group42Var2,
            Variation::Group42Var3 => ffi::Variation::Group42Var3,
            Variation::Group42Var4 => ffi::Variation::Group42Var4,
            Variation::Group42Var5 => ffi::Variation::Group42Var5,
            Variation::Group42Var6 => ffi::Variation::Group42Var6,
            Variation::Group42Var7 => ffi::Variation::Group42Var7,
            Variation::Group42Var8 => ffi::Variation::Group42Var8,
            Variation::Group50Var1 => ffi::Variation::Group50Var1,
            Variation::Group50Var2 => ffi::Variation::Group50Var2,
            Variation::Group50Var3 => ffi::Variation::Group50Var3,
            Variation::Group50Var4 => ffi::Variation::Group50Var4,
            Variation::Group51Var1 => ffi::Variation::Group51Var1,
            Variation::Group51Var2 => ffi::Variation::Group51Var2,
            Variation::Group52Var1 => ffi::Variation::Group52Var1,
            Variation::Group52Var2 => ffi::Variation::Group52Var2,
            Variation::Group60Var1 => ffi::Variation::Group60Var1,
            Variation::Group60Var2 => ffi::Variation::Group60Var2,
            Variation::Group60Var3 => ffi::Variation::Group60Var3,
            Variation::Group60Var4 => ffi::Variation::Group60Var4,

            // group 70
            Variation::Group70Var2 => ffi::Variation::Group70Var2,
            Variation::Group70Var3 => ffi::Variation::Group70Var3,
            Variation::Group70Var4 => ffi::Variation::Group70Var4,
            Variation::Group70Var5 => ffi::Variation::Group70Var5,
            Variation::Group70Var6 => ffi::Variation::Group70Var6,
            Variation::Group70Var7 => ffi::Variation::Group70Var7,
            Variation::Group70Var8 => ffi::Variation::Group70Var8,

            Variation::Group80Var1 => ffi::Variation::Group80Var1,

            Variation::Group102Var0 => ffi::Variation::Group102Var0,
            Variation::Group102Var1 => ffi::Variation::Group102Var1,

            Variation::Group110(_) => ffi::Variation::Group110,
            Variation::Group111(_) => ffi::Variation::Group111,
        }
    }
}
