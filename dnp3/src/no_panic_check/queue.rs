// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Links every operation of [`BoundedQueue`], on a queue whose capacity and history the
//! compiler can't know, so that a push, pop, front access, iteration or clear can't panic. The
//! READ selections that hold their own element types in the queue are checked where they are
//! used.

use std::hint::black_box;

use crate::util::bounded::BoundedQueue;

use super::Source;

/// one operation on the queue, chosen by `op`
#[no_panic::no_panic]
fn queue_op(queue: &mut BoundedQueue<u16>, op: u8, value: u16) {
    match op % 5 {
        0 => {
            black_box(queue.try_push(value).is_ok());
        }
        1 => {
            black_box(queue.pop_front());
        }
        2 => {
            if let Some(front) = queue.front_mut() {
                *front = value;
            }
            black_box(queue.front());
        }
        3 => {
            black_box(queue.iter().fold(0u16, |sum, x| sum.wrapping_add(*x)));
        }
        _ => queue.clear(),
    }
}

#[test]
fn queue_with_unknown_input() {
    let data = black_box(super::input());
    let mut source = Source(&data);
    let mut queue = BoundedQueue::new(source.len());
    let ops = source.len();
    for _ in 0..ops {
        queue_op(&mut queue, source.u8(), source.u16());
    }
}
