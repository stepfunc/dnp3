use dnp3::app::Variation;
use dnp3::master::{ReadHeader, ReadRequest};

use crate::ffi;

pub struct Request {
    headers: Vec<ReadHeader>,
}

impl Request {
    fn new() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    fn add(&mut self, header: ReadHeader) {
        self.headers.push(header);
    }

    pub(crate) fn build(&self) -> ReadRequest {
        ReadRequest::MultipleHeader(self.headers.clone())
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
        request.add(ReadHeader::all_objects(Variation::Group60Var2));
    }
    if class2 {
        request.add(ReadHeader::all_objects(Variation::Group60Var3));
    }
    if class3 {
        request.add(ReadHeader::all_objects(Variation::Group60Var4));
    }
    if class0 {
        request.add(ReadHeader::all_objects(Variation::Group60Var1));
    }

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_all_objects(variation: ffi::Variation) -> *mut Request {
    let mut request = Request::new();
    request.add(ReadHeader::all_objects(variation.into()));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_one_byte_range(
    variation: ffi::Variation,
    start: u8,
    stop: u8,
) -> *mut Request {
    let mut request = Request::new();
    request.add(ReadHeader::one_byte_range(variation.into(), start, stop));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_two_byte_range(
    variation: ffi::Variation,
    start: u16,
    stop: u16,
) -> *mut Request {
    let mut request = Request::new();
    request.add(ReadHeader::two_byte_range(variation.into(), start, stop));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_one_byte_limited_count(
    variation: ffi::Variation,
    count: u8,
) -> *mut Request {
    let mut request = Request::new();
    request.add(ReadHeader::one_byte_limited_count(variation.into(), count));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_new_two_byte_limited_count(
    variation: ffi::Variation,
    count: u16,
) -> *mut Request {
    let mut request = Request::new();
    request.add(ReadHeader::two_byte_limited_count(variation.into(), count));

    let request = Box::new(request);
    Box::into_raw(request)
}

pub unsafe fn request_destroy(request: *mut Request) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

pub unsafe fn request_add_one_byte_range_header(
    request: *mut Request,
    variation: ffi::Variation,
    start: u8,
    stop: u8,
) {
    if let Some(request) = request.as_mut() {
        request.add(ReadHeader::one_byte_range(variation.into(), start, stop));
    }
}

pub unsafe fn request_add_two_byte_range_header(
    request: *mut Request,
    variation: ffi::Variation,
    start: u16,
    stop: u16,
) {
    if let Some(request) = request.as_mut() {
        request.add(ReadHeader::two_byte_range(variation.into(), start, stop));
    }
}

pub unsafe fn request_add_all_objects_header(request: *mut Request, variation: ffi::Variation) {
    if let Some(request) = request.as_mut() {
        request.add(ReadHeader::all_objects(variation.into()));
    }
}

pub unsafe fn request_add_one_byte_limited_count_header(
    request: *mut Request,
    variation: ffi::Variation,
    count: u8,
) {
    if let Some(request) = request.as_mut() {
        request.add(ReadHeader::one_byte_limited_count(variation.into(), count));
    }
}

pub unsafe fn request_add_two_byte_limited_count_header(
    request: *mut Request,
    variation: ffi::Variation,
    count: u16,
) {
    if let Some(request) = request.as_mut() {
        request.add(ReadHeader::two_byte_limited_count(variation.into(), count));
    }
}

impl From<ffi::Variation> for Variation {
    fn from(from: ffi::Variation) -> Variation {
        match from {
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
            ffi::Variation::Group12Var0 => Variation::Group12Var0,
            ffi::Variation::Group12Var1 => Variation::Group12Var1,
            //ffi::Variation::Group13Var1 => Variation::Group13Var1 - TODO
            //ffi::Variation::Group13Var2 => Variation::Group13Var2 - TODO
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
            ffi::Variation::Group41Var0 => Variation::Group41Var0,
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
            /* TODO
            ffi::Variation::Group43Var1 => Variation::Group43Var1,
            ffi::Variation::Group43Var2 => Variation::Group43Var2,
            ffi::Variation::Group43Var3 => Variation::Group43Var3,
            ffi::Variation::Group43Var4 => Variation::Group43Var4,
            ffi::Variation::Group43Var5 => Variation::Group43Var5,
            ffi::Variation::Group43Var6 => Variation::Group43Var6,
            ffi::Variation::Group43Var7 => Variation::Group43Var7,
            ffi::Variation::Group43Var8 => Variation::Group43Var8,
             */
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
            ffi::Variation::Group80Var1 => Variation::Group80Var1,
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
            Variation::Group12Var0 => ffi::Variation::Group12Var0,
            Variation::Group12Var1 => ffi::Variation::Group12Var1,
            /*
            Variation::Group13Var1 => ffi::Variation::Group13Var1,
            Variation::Group13Var2 => ffi::Variation::Group13Var2,
             */
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
            Variation::Group41Var0 => ffi::Variation::Group41Var0,
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
            /* TODO
            Variation::Group43Var1 => ffi::Variation::Group43Var1,
            Variation::Group43Var2 => ffi::Variation::Group43Var2,
            Variation::Group43Var3 => ffi::Variation::Group43Var3,
            Variation::Group43Var4 => ffi::Variation::Group43Var4,
            Variation::Group43Var5 => ffi::Variation::Group43Var5,
            Variation::Group43Var6 => ffi::Variation::Group43Var6,
            Variation::Group43Var7 => ffi::Variation::Group43Var7,
            Variation::Group43Var8 => ffi::Variation::Group43Var8,
             */
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
            Variation::Group80Var1 => ffi::Variation::Group80Var1,
            Variation::Group110(_) => ffi::Variation::Group110,
            Variation::Group111(_) => ffi::Variation::Group111,
            /*
            Variation::Group112(_) => ffi::Variation::Group112,
            Variation::Group113(_) => ffi::Variation::Group113,
             */
        }
    }
}
