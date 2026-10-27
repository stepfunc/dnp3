// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The work a READ may spend selecting what it reports
//!
//! Selection counts the records it visits, and a READ whose visits exceed an allowance scaled
//! to the database is rejected. The allowances are generous multiples, so requests that are
//! merely redundant (class 0 twice) are answered, while requests that repeat database-wide
//! headers hundreds of times are not. Static and event work have separate allowances, so a
//! large event buffer can't pay for repeated static headers, or the reverse.

use crate::outstation::database::details::Visits;

/// Which allowance the work of a header counts against
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pool {
    /// static points, class 0, analog input deadbands and device attributes
    Static,
    /// event records
    Event,
}

/// A pool whose visits exceed its allowance
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct OverBudget {
    pub(crate) pool: Pool,
    pub(crate) visits: usize,
    pub(crate) allowance: usize,
}

#[derive(Copy, Clone)]
struct Allowance {
    used: usize,
    max: usize,
}

impl Allowance {
    fn new(records: usize, visits_per_record: usize) -> Self {
        Self {
            used: 0,
            max: records
                .saturating_mul(visits_per_record)
                .max(ReadBudget::MIN_VISITS),
        }
    }

    fn charge(&mut self, pool: Pool, visits: Visits) -> Result<(), OverBudget> {
        self.used = self.used.saturating_add(visits.count());
        if self.used > self.max {
            return Err(OverBudget {
                pool,
                visits: self.used,
                allowance: self.max,
            });
        }
        Ok(())
    }
}

/// The allowances of one READ, and the visits charged against them so far
pub(crate) struct ReadBudget {
    static_pool: Allowance,
    event_pool: Allowance,
}

impl ReadBudget {
    /// Visits per installed static point or attribute set
    const STATIC_VISITS_PER_RECORD: usize = 8;
    /// Visits per slot of the event buffer
    const EVENT_VISITS_PER_RECORD: usize = 16;
    /// Work too small to be worth rejecting, whatever the size of the database
    const MIN_VISITS: usize = 4096;

    /// The allowances for a database with `static_records` points and attribute sets, and an
    /// event buffer of `event_capacity` slots
    pub(crate) fn new(static_records: usize, event_capacity: usize) -> Self {
        Self {
            static_pool: Allowance::new(static_records, Self::STATIC_VISITS_PER_RECORD),
            event_pool: Allowance::new(event_capacity, Self::EVENT_VISITS_PER_RECORD),
        }
    }

    /// Charge the visits of one header, failing once its pool exceeds its allowance
    pub(crate) fn charge(&mut self, pool: Pool, visits: Visits) -> Result<(), OverBudget> {
        match pool {
            Pool::Static => self.static_pool.charge(pool, visits),
            Pool::Event => self.event_pool.charge(pool, visits),
        }
    }
}

pub(crate) fn log_over_budget(over: OverBudget) {
    let OverBudget {
        pool,
        visits,
        allowance,
    } = over;
    tracing::warn!(
        "READ rejected: selecting it visits more than {allowance} {} records ({visits})",
        match pool {
            Pool::Static => "static",
            Pool::Event => "event",
        }
    );
}
