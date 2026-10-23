// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
