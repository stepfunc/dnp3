// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use dnp3::app::attr::*;
use dnp3::app::Timestamp;
use dnp3::master::{Classes, EventClasses};
use std::ffi::CStr;

use crate::ffi;

pub struct ReadRequest {
    inner: dnp3::master::ReadRequest,
}

impl ReadRequest {
    pub(crate) fn clone_inner(&self) -> dnp3::master::ReadRequest {
        self.inner.clone()
    }

    /// replace the request with `f` of it, since its add methods take it by value
    fn update(&mut self, f: impl FnOnce(dnp3::master::ReadRequest) -> dnp3::master::ReadRequest) {
        self.inner = f(std::mem::take(&mut self.inner));
    }
}

pub struct Request {
    inner: dnp3::master::Request,
}

impl Request {
    pub(crate) fn clone_inner(&self) -> dnp3::master::Request {
        self.inner.clone()
    }

    /// replace the request with `f` of it, since its add methods take it by value
    fn update(&mut self, f: impl FnOnce(dnp3::master::Request) -> dnp3::master::Request) {
        self.inner = f(std::mem::take(&mut self.inner));
    }
}

pub unsafe fn read_request_create() -> *mut ReadRequest {
    Box::into_raw(Box::new(ReadRequest {
        inner: dnp3::master::ReadRequest::new(),
    }))
}

pub unsafe fn read_request_destroy(request: *mut ReadRequest) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

pub(crate) unsafe fn read_request_add_classes(
    request: *mut ReadRequest,
    class0: bool,
    class1: bool,
    class2: bool,
    class3: bool,
) {
    if let Some(request) = request.as_mut() {
        let classes = Classes::new(class0, EventClasses::new(class1, class2, class3));
        request.update(|r| r.add_classes(classes));
    }
}

pub(crate) unsafe fn read_request_add_device_attribute(
    request: *mut ReadRequest,
    variation: u8,
    set: u8,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_device_attribute(variation, AttrSet::new(set)));
    }
}

pub(crate) unsafe fn read_request_add_one_byte_range(
    request: *mut ReadRequest,
    variation: ffi::Variation,
    start: u8,
    stop: u8,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_one_byte_range(variation.into(), start, stop));
    }
}

pub(crate) unsafe fn read_request_add_two_byte_range(
    request: *mut ReadRequest,
    variation: ffi::Variation,
    start: u16,
    stop: u16,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_two_byte_range(variation.into(), start, stop));
    }
}

pub(crate) unsafe fn read_request_add_all_objects(
    request: *mut ReadRequest,
    variation: ffi::Variation,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_all_objects(variation.into()));
    }
}

pub(crate) unsafe fn read_request_add_one_byte_limited_count(
    request: *mut ReadRequest,
    variation: ffi::Variation,
    count: u8,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_one_byte_limited_count(variation.into(), count));
    }
}

pub(crate) unsafe fn read_request_add_two_byte_limited_count(
    request: *mut ReadRequest,
    variation: ffi::Variation,
    count: u16,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_two_byte_limited_count(variation.into(), count));
    }
}

pub unsafe fn request_create() -> *mut Request {
    Box::into_raw(Box::new(Request {
        inner: dnp3::master::Request::new(),
    }))
}

pub unsafe fn request_destroy(request: *mut Request) {
    if !request.is_null() {
        drop(Box::from_raw(request));
    }
}

pub(crate) unsafe fn request_add_classes(
    request: *mut Request,
    class0: bool,
    class1: bool,
    class2: bool,
    class3: bool,
) {
    if let Some(request) = request.as_mut() {
        let classes = Classes::new(class0, EventClasses::new(class1, class2, class3));
        request.update(|r| r.add_classes(classes));
    }
}

pub(crate) unsafe fn request_add_string_attribute(
    request: *mut Request,
    variation: u8,
    set: u8,
    value: &CStr,
) {
    if let Some(request) = request.as_mut() {
        let attr = OwnedAttribute::new(
            AttrSet::new(set),
            variation,
            OwnedAttrValue::VisibleString(value.to_string_lossy().to_string()),
        );
        request.update(|r| r.add_attribute(attr));
    }
}

pub(crate) unsafe fn request_add_uint_attribute(
    request: *mut Request,
    variation: u8,
    set: u8,
    value: u32,
) {
    if let Some(request) = request.as_mut() {
        let attr = OwnedAttribute::new(
            AttrSet::new(set),
            variation,
            OwnedAttrValue::UnsignedInt(value),
        );
        request.update(|r| r.add_attribute(attr));
    }
}

pub(crate) unsafe fn request_add_one_byte_range(
    request: *mut Request,
    variation: ffi::Variation,
    start: u8,
    stop: u8,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_one_byte_range(variation.into(), start, stop));
    }
}

pub(crate) unsafe fn request_add_two_byte_range(
    request: *mut Request,
    variation: ffi::Variation,
    start: u16,
    stop: u16,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_two_byte_range(variation.into(), start, stop));
    }
}

pub(crate) unsafe fn request_add_all_objects(request: *mut Request, variation: ffi::Variation) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_all_objects(variation.into()));
    }
}

pub(crate) unsafe fn request_add_one_byte_limited_count(
    request: *mut Request,
    variation: ffi::Variation,
    count: u8,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_one_byte_limited_count(variation.into(), count));
    }
}

pub(crate) unsafe fn request_add_two_byte_limited_count(
    request: *mut Request,
    variation: ffi::Variation,
    count: u16,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_two_byte_limited_count(variation.into(), count));
    }
}

pub(crate) unsafe fn request_add_time_and_interval(
    request: *mut Request,
    time: u64,
    interval_ms: u32,
) {
    if let Some(request) = request.as_mut() {
        request.update(|r| r.add_time_and_interval(Timestamp::new(time), interval_ms));
    }
}
