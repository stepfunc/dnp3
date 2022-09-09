use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Instant;

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

#[tokio::main(flavor = "multi_thread")]
pub async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        eprintln!("usage: perf <start port> <num pairs> <num points> <num iterations>");
        return;
    }

    let mut config = config();

    let start_port: u16 = args[1].parse().expect("start port must be a u16");
    let num_ports: u16 = args[2].parse().expect("num ports must be a u16");
    config.num_values = args[3].parse().expect("num ports must be a usize");
    let num_iterations: usize = args[4].parse().expect("num iterations must be a usize");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let port_range: std::ops::Range<u16> = start_port..start_port + num_ports;

    let mut harness = TestHarness::create(port_range, config).await;

    harness.wait_for_startup().await;

    let start = Instant::now();
    for i in 0..num_iterations {
        println!("iteration: {}", i);
        harness.run_iteration().await;
    }
    let elapsed = Instant::now() - start;
    let values = config.num_values * num_iterations * (num_ports as usize);

    println!("meas/sec: {}", (values as f64) / elapsed.as_secs_f64())
}

struct NullOutstationApplication;
impl OutstationApplication for NullOutstationApplication {}

struct NullOutstationInformation;
impl OutstationInformation for NullOutstationInformation {}

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
    _master: MasterChannel,
    // count of matching measurements received
    rx: tokio::sync::mpsc::Receiver<usize>,
    // used to update the database
    outstation: OutstationHandle,
}

impl Pair {
    const LOCALHOST: std::net::IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

    fn update_values(&mut self) {
        self.outstation.transaction(|db| {
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
            Server::new_tcp_server(LinkErrorMode::Close, SocketAddr::new(Self::LOCALHOST, port));
        let outstation = server
            .add_outstation(
                Self::get_outstation_config(config.outstation_level),
                Box::new(NullOutstationApplication),
                Box::new(NullOutstationInformation),
                DefaultControlHandler::create(),
                NullListener::create(),
                AddressFilter::Any,
            )
            .unwrap();

        // set up the database
        outstation.transaction(|db| {
            for i in 0..=config.max_index {
                db.add(
                    i,
                    Some(EventClass::Class1),
                    BinaryInputConfig::new(
                        StaticBinaryInputVariation::Group1Var2,
                        EventBinaryInputVariation::Group2Var2,
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
                    AnalogInputConfig::new(
                        StaticAnalogInputVariation::Group30Var1,
                        EventAnalogInputVariation::Group32Var3,
                        0.0,
                    ),
                );
            }
        });

        let server_handle = server.bind().await.unwrap();

        (server_handle, outstation)
    }

    async fn spawn_master(
        port: u16,
        config: TestConfig,
    ) -> (
        MasterChannel,
        Measurements,
        tokio::sync::mpsc::Receiver<usize>,
    ) {
        let mut master = dnp3::tcp::spawn_master_tcp_client(
            LinkErrorMode::Close,
            Self::get_master_config(config.master_level),
            EndpointList::single(format!("127.0.0.1:{}", port)),
            ConnectStrategy::default(),
            NullListener::create(),
        );

        let measurements = Measurements::new(config.max_index, config.num_values);
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        let handler = TestHandler {
            count: 0,
            measurements: measurements.forever(),
            tx,
        };

        // don't care about the handle
        let _ = master
            .add_association(
                Self::outstation_address(),
                Self::get_association_config(),
                Box::new(handler),
                Box::new(TestAssociationHandler),
                Box::new(TestAssociationInformation),
            )
            .await
            .unwrap();

        master.enable().await.unwrap();

        (master, measurements, rx)
    }

    fn outstation_address() -> EndpointAddress {
        EndpointAddress::try_new(10).unwrap()
    }

    fn master_address() -> EndpointAddress {
        EndpointAddress::try_new(1).unwrap()
    }

    fn get_master_config(level: DecodeLevel) -> MasterChannelConfig {
        let mut config = MasterChannelConfig::new(EndpointAddress::try_new(1).unwrap());
        config.decode_level = level;
        config
    }

    fn get_association_config() -> AssociationConfig {
        let mut config = AssociationConfig::quiet();
        config.enable_unsol_classes = EventClasses::all();
        config
    }

    fn get_outstation_config(level: DecodeLevel) -> OutstationConfig {
        let mut config = OutstationConfig::new(
            Self::outstation_address(),
            Self::master_address(),
            EventBufferConfig::all_types(100),
        );
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
    fn begin_fragment(&mut self, _read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        self.count = 0;
        MaybeAsync::ready(())
    }

    fn end_fragment(&mut self, _read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        let sender = self.tx.clone();
        let count = self.count;
        MaybeAsync::asynchronous(async move {
            let _ = sender.send(count).await;
        })
    }

    fn handle_binary_input(
        &mut self,
        _info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        for (v, i) in iter {
            if self.measurements.expect(Measurement::Binary(v, i)) {
                self.count += 1;
            }
        }
    }

    fn handle_double_bit_binary_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
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

    fn handle_analog_input(
        &mut self,
        _info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
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
        _iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        unimplemented!()
    }
}

struct TestAssociationHandler;
impl AssociationHandler for TestAssociationHandler {}

struct TestAssociationInformation;
impl AssociationInformation for TestAssociationInformation {}

// don't need to send every type
#[derive(Copy, Clone, PartialEq, Debug)]
enum Measurement {
    Binary(BinaryInput, u16),
    Counter(Counter, u16),
    Analog(AnalogInput, u16),
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

    // make sure the value and flags are consistent
    fn normalize_flags(binary: BinaryInput) -> BinaryInput {
        if binary.value {
            BinaryInput::new(
                binary.value,
                binary.flags | Flags::new(0b1000_0000),
                binary.time.unwrap(),
            )
        } else {
            let raw = binary.flags.value & 0b0111_1111;
            BinaryInput::new(binary.value, Flags::new(raw), binary.time.unwrap())
        }
    }

    fn measurement(&mut self) -> Measurement {
        match self.inner.gen_range(0..=2) {
            0 => Measurement::Binary(
                Self::normalize_flags(BinaryInput::new(
                    self.inner.gen(),
                    self.flags(),
                    self.time(),
                )),
                self.index(),
            ),
            1 => Measurement::Counter(
                Counter::new(self.inner.gen(), self.flags(), self.time()),
                self.index(),
            ),
            2 => {
                let value: u16 = self.inner.gen();
                Measurement::Analog(
                    AnalogInput::new(value as f64, self.flags(), self.time()),
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
