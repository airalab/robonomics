use libp2p::{
    core::{
        connection::ConnectionId,
        upgrade::{OutboundUpgrade, ReadyUpgrade},
        ConnectedPoint,
    },
    swarm::{
        handler::{
            ConnectionHandler, ConnectionHandlerEvent, InboundUpgradeSend, IntoConnectionHandler,
        },
        ConnectionHandlerUpgrErr, KeepAlive, NegotiatedSubstream, NetworkBehaviour,
        NetworkBehaviourAction, PollParameters, SubstreamProtocol,
    },
    PeerId,
};
use std::{
    collections::VecDeque,
    sync::Arc,
    task::{Context, Poll},
};
use void::Void;

const PROTOCOL_NAME: &[u8] = b"/robonomics/ros/1.0.0";
const MAX_SUBSTREAM_CREATION: usize = 5;

pub struct Ros {
    events: VecDeque<RosEvent>,
}

impl Ros {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }
    pub fn publish(&mut self, peer_id: PeerId, message: String) {
        self.events
            .push_front(RosEvent::Publish { peer_id, message });
    }
    pub fn subscribe(&mut self, peer_id: PeerId, topic: String) {
        self.events
            .push_front(RosEvent::Subscribe { peer_id, topic });
    }
}

/// Event that can be emitted by the ROS behaviour.
#[derive(Debug)]
pub enum RosEvent {
    Publish { peer_id: PeerId, message: String },
    Subscribe { peer_id: PeerId, topic: String },
}

impl NetworkBehaviour for Ros {
    type ConnectionHandler = RosHandler;
    type OutEvent = RosEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        RosHandler::new()
    }

    fn inject_event(&mut self, _: PeerId, _: ConnectionId, event: RosHandlerEvent) {
        println!("RosHandlerEvent from behaviour! {:?}", event);
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        endpoint: &ConnectedPoint,
        _: Option<&Vec<libp2p::Multiaddr>>,
        other_established: usize,
    ) {
        println!(
            "Connection established {:?} {:?} {:?} {:?}",
            peer_id, connection_id, endpoint, other_established
        );
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        endpoint: &ConnectedPoint,
        _: <Self::ConnectionHandler as IntoConnectionHandler>::Handler,
        remaining_established: usize,
    ) {
        println!(
            "Connection closed {:?} {:?} {:?} {:?}",
            peer_id, connection_id, endpoint, remaining_established
        );
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        if let Some(event) = self.events.pop_back() {
            Poll::Ready(NetworkBehaviourAction::GenerateEvent(event))

            // Poll::Ready(NetworkBehaviourAction::NotifyHandler {
            //     peer_id: *peer_id,
            //     handler: libp2p::swarm::behaviour::NotifyHandler::One(RosHandler),
            //     event: RosEvent::Publish { peer_id: *peer_id },
            // })
        } else {
            Poll::Pending
        }
    }
}

enum SubstreamState {
    Ready(NegotiatedSubstream),
    NotReady,
}

/// Protocol Handler that manages a single long-lived substream with a peer.
pub struct RosHandler {
    outbound_substream_establishing: bool,
    outbound_substreams_created: usize,
    inbound_substreams_created: usize,

    // events: VecDeque<RosHandlerEvent>,
    /// The single long-lived outbound substream.
    outbound_substream: Option<SubstreamState>,

    /// The single long-lived inbound substream.
    inbound_substream: Option<SubstreamState>,
}

impl RosHandler {
    pub fn new() -> Self {
        Self {
            outbound_substream_establishing: false,
            outbound_substreams_created: 0,
            inbound_substreams_created: 0,
            // events: VecDeque::new(),
            outbound_substream: None,
            inbound_substream: None,
        }
    }
}

/// The event emitted by the Handler.
/// This informs the behaviour of various events created by the handler.
#[derive(Debug)]
pub enum RosHandlerEvent {
    Publish,
    Subscribe,
    // Subscribe {
    //     peer_id: PeerId,
    //     topic: String,
    // },
    // Publish {
    //     peer_id: PeerId,
    //     message: String,
    //     topic: String,
    // },
}

impl ConnectionHandler for RosHandler {
    type InEvent = RosEvent;
    type OutEvent = RosHandlerEvent;
    type Error = crate::error::Error;
    type InboundProtocol = ReadyUpgrade<&'static [u8]>;
    type OutboundProtocol = ReadyUpgrade<&'static [u8]>;
    type OutboundOpenInfo = ();
    type InboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ())
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        substream: NegotiatedSubstream,
        _: Self::InboundOpenInfo,
    ) {
        self.inbound_substreams_created += 1;
        self.inbound_substream = Some(SubstreamState::Ready(substream));
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: NegotiatedSubstream,
        _: Self::OutboundOpenInfo,
    ) {
        self.outbound_substream_establishing = false;
        self.outbound_substreams_created += 1;

        if self.outbound_substream.is_some() {
            println!("Established an outbound substream with one already available");
        } else {
            self.outbound_substream = Some(SubstreamState::Ready(substream));
        }
    }

    fn inject_event(&mut self, event: RosEvent) {
        println!("Event4! {:?}", event);
    }

    fn inject_listen_upgrade_error(
        &mut self,
        _: Self::InboundOpenInfo,
        _: ConnectionHandlerUpgrErr<<Self::InboundProtocol as InboundUpgradeSend>::Error>,
    ) {
        println!("Error!!!!");
    }

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        err: ConnectionHandlerUpgrErr<
            <Self::OutboundProtocol as OutboundUpgrade<NegotiatedSubstream>>::Error,
        >,
    ) {
        err.map_upgrade_err(|e| {
            println!("Upgrade error: {:?}", e);
            e
        });
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<
        ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::OutEvent,
            Self::Error,
        >,
    > {
        if self.inbound_substreams_created > MAX_SUBSTREAM_CREATION {
            return Poll::Ready(ConnectionHandlerEvent::Close(crate::error::Error::Other(
                "Max inbound substreams".to_string(),
            )));
        }

        if !self.outbound_substream_establishing && self.outbound_substream.is_none() {
            if self.outbound_substreams_created >= MAX_SUBSTREAM_CREATION {
                return Poll::Ready(ConnectionHandlerEvent::Close(crate::error::Error::Other(
                    "Max outbound substreams".to_string(),
                )));
            }
            self.outbound_substream_establishing = true;
            return Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ()),
            });
        }

        // !!!
        // return Poll::Ready(ConnectionHandlerEvent::Custom(RosHandlerEvent::Publish));

        loop {
            match self.inbound_substream.take() {
                Some(SubstreamState::Ready(substream)) => {
                    println!("Ready inbound");
                    // match substream.poll_next_unpin(cx) {
                    //     Poll::Pending => {
                    //         self.inbound_substream = Some(SubstreamState::Ready(substream));
                    //         break;
                    //     }
                    //     _ => {}
                    // }
                }
                _ => {
                    self.inbound_substream = None;
                    break;
                }
            }
        }

        loop {
            match self.outbound_substream.take() {
                Some(SubstreamState::Ready(stream)) => {
                    println!("Ready outbound");
                }
                _ => {
                    self.inbound_substream = None;
                    break;
                }
            }
        }

        Poll::Pending
    }
}
