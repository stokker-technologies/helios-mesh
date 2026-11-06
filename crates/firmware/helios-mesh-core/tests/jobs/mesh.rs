/* ***
 *
 * Helios Mesh
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * ***
 */

// Exercise The Firmware Coordinator With Deterministic Multi-Node Packet Arrivals.

/* === Modules === */

#[path = "../support/radio.rs"]
mod radio;

/* === Re-Exports === */

/* === Imports === */

// Standard Library Imports
use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    pin::pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

// Third Party Imports
use embassy_futures::{
    block_on,
    join::join,
    select::{Either, select},
};
use embassy_time::{Duration, Instant, MockDriver, Timer};
use embedded_storage::nor_flash::{ErrorType, NorFlash, NorFlashErrorKind, ReadNorFlash};

// Local Imports
use helios_mesh_core::{
    boot::BootReport,
    identity::Identity,
    mesh::{Coordinator, link::Link, retry::INTERACTIVE},
    node::{
        Ctx, Handle, Node, Origin, Outcome, Runtime, Submission,
        services::{
            binding::Binding,
            replay::{Cache, Lookup},
        },
    },
    platform::{Platform, led},
};
use helios_mesh_protocol::{
    error::Error,
    gateway,
    node::{self, AuxGpioAccess, AuxGpioMode, FIRMWARE_VERSION, NodeDescription, NodeId, NodeInfo, RequestId, SessionId},
    nodes::NodeType,
    wire::{
        PROTOCOL_VERSION, Request, Response,
        mesh::{Envelope, Message},
    },
};
use radio::{Air, Arrival, Delay, Radio};

/* === Constants === */

const GATEWAY: NodeId = NodeId(100_000);
const SESSION: SessionId = SessionId(7);

/* === Macros === */

/* === Type Aliases === */

/* === Structs === */

struct Gateway;
struct Flash;
struct Board;

struct Leaf {
    id: NodeId,
    binding: Binding,
    replay: Cache<6>,
    writes: usize,
}

/* === Enums === */

/* === Traits === */

/* === Implementations === */

impl Leaf {
    fn new(id: u32) -> Self {
        Self { id: NodeId(id), binding: Binding::new(), replay: Cache::new(), writes: 0 }
    }

    fn response(&mut self, packet: &Envelope) -> Envelope {
        let Message::Request { request, .. } = &packet.message else { unreachable!() };
        let write = matches!(request, Request::Node(node::Request::ConfigureAuxGpio(_)));
        let response = if let Err(gateway) = self.binding.admit(packet.source, packet.session, write, Instant::now().as_millis()) {
            Error::BoundElsewhere { gateway }.into()
        } else if packet.destination != NodeId::BROADCAST {
            match self.replay.lookup(packet.source, packet.session, packet.request_id, request) {
                Lookup::Replay(response) => response.clone(),
                Lookup::Conflict => Error::DuplicateRequestConflict.into(),
                Lookup::Miss => {
                    let response = self.execute(request);
                    self.replay.insert(packet.source, packet.session, packet.request_id, request.clone(), response.clone());
                    response
                }
            }
        } else {
            self.execute(request)
        };
        Envelope {
            version: PROTOCOL_VERSION,
            source: self.id,
            session: SessionId(9),
            destination: packet.source,
            request_id: packet.request_id,
            message: Message::Response { response, event: None },
        }
    }

    fn execute(&mut self, request: &Request) -> Response {
        match request {
            Request::Node(node::Request::Describe(_)) => node::Response::Description(NodeDescription {
                info: NodeInfo { node_type: NodeType::Sidecar, id: self.id, firmware: FIRMWARE_VERSION },
                satellites: heapless::Vec::new(),
            })
            .into(),
            Request::Node(node::Request::Status(_)) => node::Response::Status(node::NodeStatus {
                info: NodeInfo { node_type: NodeType::Sidecar, id: self.id, firmware: FIRMWARE_VERSION },
                auxiliary_gpio: [AuxGpioMode::HighImpedance; 4],
                auxiliary_gpio_access: [AuxGpioAccess::PushPull; 4],
            })
            .into(),
            Request::Node(node::Request::ConfigureAuxGpio(query)) => {
                self.writes += 1;
                node::Response::AuxGpioConfigured { gpio: query.gpio, mode: query.mode }.into()
            }
            _ => Error::Unsupported.into(),
        }
    }
}

impl Node for Gateway {
    type Request = Request;
    type Config = ();
    const NODE_TYPE: NodeType = NodeType::Gateway;
    fn handle_request(&mut self, _: &mut Ctx<'_, Self>, _: Request) -> impl Future<Output = Outcome> {
        core::future::ready(Outcome::reply(Error::Unsupported))
    }
}

impl ErrorType for Flash {
    type Error = NorFlashErrorKind;
}
impl ReadNorFlash for Flash {
    const READ_SIZE: usize = 1;
    fn read(&mut self, _: u32, _: &mut [u8]) -> Result<(), Self::Error> {
        unreachable!("this node has no configuration")
    }
    fn capacity(&self) -> usize {
        0
    }
}
impl NorFlash for Flash {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = 1;
    fn write(&mut self, _: u32, _: &[u8]) -> Result<(), Self::Error> {
        unreachable!()
    }
    fn erase(&mut self, _: u32, _: u32) -> Result<(), Self::Error> {
        unreachable!()
    }
}
impl Platform for Board {
    fn auxiliary_gpio_access(&self) -> [AuxGpioAccess; 4] {
        [AuxGpioAccess::NotAllocated; 4]
    }
    fn auxiliary_gpio_modes(&self) -> [AuxGpioMode; 4] {
        [AuxGpioMode::HighImpedance; 4]
    }
    fn restart(&mut self, _: Option<node::Bootloader>) {
        unreachable!()
    }
    fn led(&self) -> led::Handle {
        static SIGNALS: led::Signals = led::Signals::new();
        SIGNALS.split().0
    }
}

/* === Public Functions === */

/* === Private Functions === */

fn drive<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        MockDriver::get().advance(Duration::from_millis(1));
    }
}

fn packet(source: NodeId, request_id: RequestId) -> Envelope {
    Envelope {
        version: PROTOCOL_VERSION,
        source,
        session: SESSION,
        destination: GATEWAY,
        request_id,
        message: Message::Response { response: node::Response::Polled.into(), event: None },
    }
}

async fn usb(node: Handle<Gateway>, id: u32, destination: NodeId, request: Request) -> Response {
    node.submit(Submission { origin: Origin::Usb, version: PROTOCOL_VERSION, request_id: RequestId(id), destination, request }).await;
    let reply = node.next_usb_response().await;
    assert_eq!(reply.request_id, RequestId(id));
    reply.response
}

fn gateway(link: Link<Radio, Delay>) -> (impl Future<Output = ()>, Handle<Gateway>) {
    let coordinator = Box::leak(Box::new(Coordinator::new(link)));
    let (radio, handle) = coordinator.split();
    let runtime =
        Runtime::compose(Identity { node: GATEWAY, session: SESSION }, Flash, Board, BootReport::default(), Gateway).forward_to(handle);
    let (node, handle) = Box::leak(Box::new(runtime)).split();
    (
        async move {
            join(node.run(), radio.run(handle)).await;
            unreachable!()
        },
        handle,
    )
}

/* === Tests === */

#[test]
fn exchange_survives_corruption() {
    let mut leaves = [Leaf::new(100_001), Leaf::new(100_002), Leaf::new(100_003)];
    let air = Rc::new(RefCell::new(Air {
        sent: Vec::new(),
        arrivals: VecDeque::new(),
        script: Box::new(move |request, arrivals| {
            for (index, leaf) in leaves.iter_mut().enumerate() {
                if request.destination != leaf.id && request.destination != NodeId::BROADCAST {
                    continue;
                }
                if request.destination != NodeId::BROADCAST {
                    arrivals.push_back(Arrival::corrupt(5));
                }
                let delay = if request.destination == NodeId::BROADCAST { 20 + u64::try_from(index).unwrap() * 25 } else { 10 };
                arrivals.push_back(Arrival::packet(delay, &leaf.response(request)));
            }
        }),
    }));
    let (background, handle) = gateway(block_on(radio::link(air)));
    drive(async {
        let workflow = async {
            Timer::after(Duration::from_millis(660)).await;
            for id in [NodeId(100_001), NodeId(100_002), NodeId(100_003)] {
                let response = usb(handle, 1, id, node::Describe.into()).await;
                assert!(matches!(response, Response::Node(node::Response::Description(description)) if description.info.id == id));
            }
        };
        assert!(matches!(select(background, workflow).await, Either::Second(())));
    });
}

#[test]
fn mesh_backpressure() {
    let mut leaves = [Leaf::new(100_001), Leaf::new(100_002), Leaf::new(100_003)];
    let air = Rc::new(RefCell::new(Air {
        sent: Vec::new(),
        arrivals: VecDeque::new(),
        script: Box::new(move |request, arrivals| {
            for (index, leaf) in leaves.iter_mut().enumerate() {
                if request.destination != leaf.id && request.destination != NodeId::BROADCAST {
                    continue;
                }
                let delay = if request.destination == NodeId::BROADCAST { 20 + u64::try_from(index).unwrap() * 25 } else { 10 };
                arrivals.push_back(Arrival::packet(delay, &leaf.response(request)));
            }
        }),
    }));
    let (background, handle) = gateway(block_on(radio::link(air)));
    drive(async {
        let workflow = async {
            Timer::after(Duration::from_millis(660)).await;
            // The Firmware Has Eight Shared Forward Slots, With No Per-Node Credits.
            for index in 0..9 {
                handle
                    .submit(Submission {
                        origin: Origin::Usb,
                        version: PROTOCOL_VERSION,
                        request_id: RequestId(100 + index),
                        destination: NodeId(100_001 + index % 3),
                        request: node::Describe.into(),
                    })
                    .await;
            }
            let mut completed = Vec::new();
            let mut busy = Vec::new();
            for _ in 0..9 {
                let response = handle.next_usb_response().await;
                match response.response {
                    Response::Error(Error::Busy) => busy.push(response.request_id),
                    Response::Node(node::Response::Description(description)) => {
                        assert_eq!(description.info.id, NodeId(100_001 + (response.request_id.0 - 100) % 3));
                        completed.push(response.request_id);
                    }
                    other => panic!("unexpected response: {other:?}"),
                }
            }
            completed.sort_by_key(|id| id.0);
            assert_eq!(completed, (100..108).map(RequestId).collect::<Vec<_>>());
            assert_eq!(busy, [RequestId(108)]);
            let response = usb(handle, 109, NodeId(100_003), node::Describe.into()).await;
            assert!(matches!(response, Response::Node(node::Response::Description(description)) if description.info.id == NodeId(100_003)));
        };
        assert!(matches!(select(background, workflow).await, Either::Second(())));
    });
}

#[test]
fn mesh_scales() {
    for leaf_count in [1, 2, 3, 7, 16] {
        let leaves = Rc::new(RefCell::new((1..=leaf_count).map(|id| Leaf::new(100_000 + id)).collect::<Vec<_>>()));
        let endpoints = leaves.clone();
        let air = Rc::new(RefCell::new(Air {
            sent: Vec::new(),
            arrivals: VecDeque::new(),
            script: Box::new(move |request, arrivals| {
                for (index, leaf) in endpoints.borrow_mut().iter_mut().enumerate() {
                    if request.destination == leaf.id || request.destination == NodeId::BROADCAST {
                        let delay = if request.destination == NodeId::BROADCAST { 20 + u64::try_from(index).unwrap() * 25 } else { 10 };
                        let drop_reply = leaf.writes == 0
                            && matches!(
                                request.message,
                                Message::Request { request: Request::Node(node::Request::ConfigureAuxGpio(_)), .. }
                            );
                        let response = leaf.response(request);
                        if !drop_reply {
                            arrivals.push_back(Arrival::packet(delay, &response));
                        }
                    }
                }
            }),
        }));
        let link = block_on(radio::link(air.clone()));
        let (background, handle) = gateway(link);
        drive(async {
            let workflow = async {
                Timer::after(Duration::from_millis(660)).await;
                let mut after = None;
                let mut ids = Vec::new();
                loop {
                    let Response::Gateway(gateway::Response::Nodes(page)) = usb(handle, 1, GATEWAY, gateway::Nodes { after }.into()).await
                    else {
                        panic!("roster failed")
                    };
                    ids.extend(page.nodes.iter().map(|summary| summary.info.id));
                    after = page.next_after;
                    if after.is_none() {
                        break;
                    }
                }
                assert_eq!(ids, (0..=leaf_count).map(|id| NodeId(100_000 + id)).collect::<Vec<_>>());
                for id in ids.into_iter().filter(|id| *id != GATEWAY) {
                    let response = usb(handle, 2, id, node::Describe.into()).await;
                    assert!(
                        matches!(&response, Response::Node(node::Response::Description(description)) if description.info.id == id),
                        "node {id:?}: {response:?}; air={:?}",
                        air.borrow().sent
                    );
                    let response =
                        usb(handle, 3, id, node::ConfigureAuxGpio { gpio: node::AuxGpioId(1), mode: AuxGpioMode::DriveLow }.into()).await;
                    assert!(matches!(response, Response::Node(node::Response::AuxGpioConfigured { .. })));
                }
            };
            assert!(matches!(select(background, workflow).await, Either::Second(())));
        });
        assert!(leaves.borrow().iter().all(|leaf| leaf.writes == 1));
        let air = air.borrow();
        for leaf in leaves.borrow().iter() {
            let requests = air.sent.iter().map(|(_, request)| request).filter(|request| request.destination == leaf.id).collect::<Vec<_>>();
            assert!(requests.windows(2).all(|pair| pair[1].request_id.0 > pair[0].request_id.0 || pair[1] == pair[0]));
            let writes = requests
                .iter()
                .filter(|packet| {
                    matches!(packet.message, Message::Request { request: Request::Node(node::Request::ConfigureAuxGpio(_)), .. })
                })
                .count();
            assert_eq!(writes, 2, "a lost reply must retry the original admission");
        }
        println!("{} total nodes: roster, addressing, writes and request IDs passed", leaf_count + 1);
    }
}

#[test]
fn discovery_survives_corruption() {
    for leaf_count in [2, 3, 7, 16] {
        let mut leaves = (1..=leaf_count).map(|id| Leaf::new(100_000 + id)).collect::<Vec<_>>();
        let air = Rc::new(RefCell::new(Air {
            sent: Vec::new(),
            arrivals: VecDeque::new(),
            script: Box::new(move |request, arrivals| {
                if request.destination == NodeId::BROADCAST {
                    arrivals.push_back(Arrival::corrupt(5));
                    for (index, leaf) in leaves.iter_mut().enumerate() {
                        arrivals.push_back(Arrival::packet(20 + u64::try_from(index).unwrap() * 25, &leaf.response(request)));
                    }
                }
            }),
        }));
        let (background, handle) = gateway(block_on(radio::link(air)));
        drive(async {
            let workflow = async {
                Timer::after(Duration::from_millis(660)).await;
                let mut after = None;
                let mut count = 0;
                loop {
                    let Response::Gateway(gateway::Response::Nodes(page)) = usb(handle, 1, GATEWAY, gateway::Nodes { after }.into()).await
                    else {
                        panic!("roster failed")
                    };
                    count += page.nodes.len();
                    after = page.next_after;
                    if after.is_none() {
                        break;
                    }
                }
                assert_eq!(count, usize::try_from(leaf_count).unwrap() + 1, "a corrupt frame must not discard later replies");
            };
            assert!(matches!(select(background, workflow).await, Either::Second(())));
        });
    }
}

#[test]
fn exchange_deadline_authoritative() {
    let mut leaf = Leaf::new(100_001);
    let air = Rc::new(RefCell::new(Air {
        sent: Vec::new(),
        arrivals: VecDeque::new(),
        script: Box::new(move |request, arrivals| {
            if request.destination == NodeId::BROADCAST {
                arrivals.push_back(Arrival::packet(20, &leaf.response(request)));
            } else if matches!(request.message, Message::Request { request: Request::Node(node::Request::Describe(_)), .. }) {
                // Another Pair's Traffic Must Not Renew This Attempt's 200ms Budget.
                for delay in (50..=500).step_by(50) {
                    arrivals.push_back(Arrival::packet(delay, &packet(NodeId(100_002), RequestId(999))));
                }
            } else {
                arrivals.push_back(Arrival::packet(10, &leaf.response(request)));
            }
        }),
    }));
    let observed = air.clone();
    let (background, handle) = gateway(block_on(radio::link(air)));
    drive(async {
        let workflow = async {
            Timer::after(Duration::from_millis(660)).await;
            assert_eq!(usb(handle, 5, NodeId(100_001), node::Describe.into()).await, Error::Unavailable.into());
            let requests = observed
                .borrow()
                .sent
                .iter()
                .filter(|(_, packet)| {
                    packet.destination == NodeId(100_001)
                        && matches!(packet.message, Message::Request { request: Request::Node(node::Request::Describe(_)), .. })
                })
                .map(|(at, packet)| (*at, packet.request_id))
                .collect::<Vec<_>>();
            assert_eq!(requests.len(), usize::from(INTERACTIVE.attempts));
            assert!(requests.iter().all(|(_, id)| *id == requests[0].1));
            assert!(requests.windows(2).all(|pair| pair[1].0 - pair[0].0 <= Duration::from_millis(225)));
        };
        assert!(matches!(select(background, workflow).await, Either::Second(())));
    });
}
