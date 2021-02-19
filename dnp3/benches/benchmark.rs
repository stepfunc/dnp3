use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{Rng, SeedableRng};

use dnp3::app::measurement::*;
use dnp3::app::*;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::master::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;
use dnp3::tcp::*;

fn config() -> TestConfig {
    TestConfig {
        outstation_level: DecodeLevel::nothing(),
        master_level: DecodeLevel::nothing(),
        num_values: 100,
        max_index: 10,
    }
}

// the ports used... the number of ports determines the number of parallel entries
const PORT_RANGE_16: std::ops::Range<u16> = 50000..50016;

struct TestInstance {
    runtime: tokio::runtime::Runtime,
    harness: TestHarness,
}

impl TestInstance {
    fn create(ports: std::ops::Range<u16>) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let mut harness = runtime.block_on(TestHarness::create(ports, config()));

        runtime.block_on(harness.wait_for_startup());

        Self { runtime, harness }
    }

    fn run_iteration(&mut self) {
        self.runtime.block_on(self.harness.run_iteration());
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let mut instance = TestInstance::create(PORT_RANGE_16);
    c.bench_function("16 sessions", |b| {
        b.iter(|| {
            instance.run_iteration();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

#[derive(Copy, Clone)]
struct TestConfig {
    outstation_level: DecodeLevel,
    master_level: DecodeLevel,
    num_values: usize,
    max_index: u16,
}

struct TestHarness {
    pairs: Vec<Pair>,
}

impl TestHarness {
    async fn create(ports: std::ops::Range<u16>, config: TestConfig) -> Self {
        let mut pairs = Vec::new();
        for port in ports {
            pairs.push(Pair::spawn(port, config).await)
        }
        Self { pairs }
    }

    async fn wait_for_startup(&mut self) {
        for pair in &mut self.pairs {
            pair.wait_for_null_unsolicited().await;
        }
    }

    async fn run_iteration(&mut self) {
        for pair in &mut self.pairs {
            pair.update_values();
        }

        for pair in &mut self.pairs {
            pair.wait_for_update().await;
        }
    }
}

struct Pair {
    // measurements exchanged on each iteration
    values: Measurements,
    // have to hold onto this to keep TCP server alive
    _server: ServerHandle,
    // have to hold onto this to keep master alive
    _master: MasterHandle,
    // count of matching measurements received
    rx: tokio::sync::mpsc::Receiver<usize>,
    // used to update the database
    outstation: OutstationHandle,
}

impl Pair {
    const LOCALHOST: std::net::IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    fn update_values(&mut self) {
        self.outstation.database.transaction(|db| {
            for x in self.values.once() {
                x.apply(db)
            }
        })
    }

    async fn wait_for_null_unsolicited(&mut self) {
        assert_eq!(self.rx.recv().await.unwrap(), 0);
    }

    async fn wait_for_update(&mut self) {
        assert_eq!(self.rx.recv().await.unwrap(), self.values.len());
    }

    async fn spawn(port: u16, config: TestConfig) -> Self {
        let (server, outstation) = Self::spawn_outstation(port, config).await;
        let (master, measurements, rx) = Self::spawn_master(port, config).await;

        Self {
            values: measurements,
            _server: server,
            rx,
            _master: master,
            outstation,
        }
    }

    async fn spawn_outstation(port: u16, config: TestConfig) -> (ServerHandle, OutstationHandle) {
        let mut server =
            TcpServer::new(LinkErrorMode::Close, SocketAddr::new(Self::LOCALHOST, port));
        let (outstation, task) = server
            .add_outstation(
                Self::get_outstation_config(config.outstation_level),
                EventBufferConfig::all_types(100),
                DefaultOutstationApplication::create(),
                DefaultOutstationInformation::create(),
                DefaultControlHandler::create(),
                AddressFilter::Any,
            )
            .unwrap();
        tokio::spawn(task);

        // set up the database
        outstation.database.transaction(|db| {
            for i in 0..=config.max_index {
                db.add(
                    i,
                    Some(EventClass::Class1),
                    BinaryConfig::new(
                        StaticBinaryVariation::Group1Var2,
                        EventBinaryVariation::Group2Var2,
                    ),
                );
                db.add(
                    i,
                    Some(EventClass::Class1),
                    CounterConfig::new(
                        StaticCounterVariation::Group20Var2,
                        EventCounterVariation::Group22Var5,
                        0,
                    ),
                );
                db.add(
                    i,
                    Some(EventClass::Class1),
                    AnalogConfig::new(
                        StaticAnalogVariation::Group30Var1,
                        EventAnalogVariation::Group32Var3,
                        0.0,
                    ),
                );
            }
        });

        let (server_handle, task) = server.bind().await.unwrap();
        tokio::spawn(task);

        (server_handle, outstation)
    }

    async fn spawn_master(
        port: u16,
        config: TestConfig,
    ) -> (
        MasterHandle,
        Measurements,
        tokio::sync::mpsc::Receiver<usize>,
    ) {
        let mut master = dnp3::tcp::spawn_master_tcp_client(
            LinkErrorMode::Close,
            Self::get_master_config(config.master_level),
            EndpointList::single(format!("127.0.0.1:{}", port)),
            Listener::None,
        );

        let measurements = Measurements::new(config.max_index, config.num_values);
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        let handler = TestHandler {
            count: 0,
            measurements: measurements.forever(),
            tx,
        };

        // don't are about the handle
        let _ = master
            .add_association(
                Self::outstation_address(),
                Self::get_association_config(),
                Box::new(handler),
            )
            .await
            .unwrap();

        (master, measurements, rx)
    }

    fn outstation_address() -> EndpointAddress {
        EndpointAddress::from(10).unwrap()
    }

    fn master_address() -> EndpointAddress {
        EndpointAddress::from(1).unwrap()
    }

    fn get_master_config(level: DecodeLevel) -> MasterConfig {
        MasterConfig::new(
            Self::master_address(),
            level,
            ReconnectStrategy::default(),
            Timeout::from_secs(5).unwrap(),
        )
    }

    fn get_association_config() -> AssociationConfig {
        let mut config = AssociationConfig::quiet(RetryStrategy::default());
        config.enable_unsol_classes = EventClasses::all();
        config
    }

    fn get_outstation_config(level: DecodeLevel) -> OutstationConfig {
        let mut config = OutstationConfig::new(Self::outstation_address(), Self::master_address());
        config.decode_level = level;
        config
    }
}

struct TestHandler {
    count: usize,
    measurements: CyclicMeasurementIterator,
    tx: tokio::sync::mpsc::Sender<usize>,
}

impl ReadHandler for TestHandler {
    fn begin_fragment(&mut self, _header: ResponseHeader) {
        self.count = 0;
    }

    fn end_fragment(&mut self, _header: ResponseHeader) {
        self.tx.try_send(self.count).unwrap();
    }

    fn handle_binary(&mut self, _info: HeaderInfo, iter: &mut dyn Iterator<Item = (Binary, u16)>) {
        for (v, i) in iter {
            if self.measurements.expect(Measurement::Binary(v, i)) {
                self.count += 1;
            }
        }
    }

    fn handle_double_bit_binary(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
    ) {
        unimplemented!()
    }

    fn handle_binary_output_status(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        unimplemented!()
    }

    fn handle_counter(
        &mut self,
        _info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (Counter, u16)>,
    ) {
        for (v, i) in iter {
            if self.measurements.expect(Measurement::Counter(v, i)) {
                self.count += 1;
            }
        }
    }

    fn handle_frozen_counter(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        unimplemented!()
    }

    fn handle_analog(&mut self, _info: HeaderInfo, iter: &mut dyn Iterator<Item = (Analog, u16)>) {
        for (v, i) in iter {
            if self.measurements.expect(Measurement::Analog(v, i)) {
                self.count += 1;
            }
        }
    }

    fn handle_analog_output_status(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        unimplemented!()
    }

    fn handle_octet_string<'a>(
        &mut self,
        _info: HeaderInfo,
        _iter: &'a mut dyn Iterator<Item = (Bytes<'a>, u16)>,
    ) {
        unimplemented!()
    }
}

impl AssociationHandler for TestHandler {
    fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }

    fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }

    fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }
}

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
    fn len(&self) -> usize {
        self.inner.len()
    }
    fn new(max_index: u16, count: usize) -> Self {
        let mut rand = Random::new(max_index);
        let mut values = Vec::new();
        for _ in 0..count {
            values.push(rand.measurement());
        }
        Self {
            inner: std::sync::Arc::new(values),
        }
    }

    fn once(&self) -> impl Iterator<Item = &Measurement> {
        self.inner.iter()
    }

    fn forever(&self) -> CyclicMeasurementIterator {
        CyclicMeasurementIterator {
            next: 0,
            list: self.clone(),
        }
    }
}

struct CyclicMeasurementIterator {
    next: usize,
    list: Measurements,
}

impl CyclicMeasurementIterator {
    fn next(&mut self) -> Measurement {
        let index = self.next;
        self.next = (self.next + 1) % self.list.inner.len();
        self.list.inner[index]
    }

    fn expect(&mut self, actual: Measurement) -> bool {
        let expected = self.next();
        if expected == actual {
            true
        } else {
            tracing::error!("expected: {:?} actual: {:?}", expected, actual);
            false
        }
    }
}
