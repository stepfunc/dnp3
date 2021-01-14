use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dnp3::app::measurement::{Analog, Binary, Counter, Time};
use dnp3::outstation::database::{Database, EventMode, Update, UpdateOptions};

use dnp3::app::flags::Flags;
use dnp3::app::types::Timestamp;
use rand::{Rng, SeedableRng};

// don't need to send every type
#[derive(Copy, Clone, PartialEq, Debug)]
enum Measurement {
    Binary(Binary, u16),
    Counter(Counter, u16),
    Analog(Analog, u16),
}

impl Measurement {
    const OPTIONS: UpdateOptions = UpdateOptions::new(false, EventMode::Force);

    fn apply(&self, db: &mut Database) {
        match self {
            Measurement::Binary(v, i) => db.update(*i, v, Self::OPTIONS),
            Measurement::Counter(v, i) => db.update(*i, v, Self::OPTIONS),
            Measurement::Analog(v, i) => db.update(*i, v, Self::OPTIONS),
        };
    }
}

struct Random {
    max_index: u16,
    inner: rand::rngs::StdRng,
}

impl Random {
    fn new(max_index: u16) -> Self {
        Self {
            max_index,
            inner: rand::rngs::StdRng::seed_from_u64(0),
        }
    }

    fn measurement(&mut self) -> Measurement {
        match self.inner.gen_range(0..=2) {
            0 => Measurement::Binary(
                Binary::new(self.inner.gen(), self.flags(), self.time()).normalize(),
                self.index(),
            ),
            1 => Measurement::Counter(
                Counter::new(self.inner.gen(), self.flags(), self.time()),
                self.index(),
            ),
            2 => {
                let value: u16 = self.inner.gen();
                Measurement::Analog(
                    Analog::new(value as f64, self.flags(), self.time()),
                    self.index(),
                )
            }
            _ => unreachable!(),
        }
    }

    fn index(&mut self) -> u16 {
        self.inner.gen_range(0..self.max_index)
    }

    fn flags(&mut self) -> Flags {
        Flags::new(self.inner.gen())
    }

    fn time(&mut self) -> Time {
        Time::Synchronized(Timestamp::new(self.inner.gen()))
    }
}

#[derive(Clone)]
struct Measurements {
    inner: std::sync::Arc<Vec<Measurement>>,
}

impl Measurements {
    fn iterator(&self) -> MeasurementIterator {
        MeasurementIterator {
            next: 0,
            list: self.clone(),
        }
    }
}

struct MeasurementIterator {
    next: usize,
    list: Measurements,
}

impl MeasurementIterator {
    fn next(&mut self) -> Measurement {
        let index = self.next;
        self.next = (self.next + 1) % self.list.inner.len();
        self.list.inner[index]
    }
}

fn generate_measurements(max_index: u16, count: usize) -> Measurements {
    let mut rand = Random::new(max_index);
    let mut values = Vec::new();
    for _ in 0..count {
        values.push(rand.measurement());
    }
    Measurements {
        inner: std::sync::Arc::new(values),
    }
}

fn fibonacci(_: u32) {}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
