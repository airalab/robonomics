use crate::pubsub::Pubsub;
use libp2p::{
    core::{
        connection::ConnectionId,
        either::EitherError,
        upgrade::{InboundUpgrade, NegotiationError, OutboundUpgrade, ReadyUpgrade},
        ConnectedPoint, UpgradeError, UpgradeInfo,
    },
    gossipsub::Gossipsub,
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
    iter,
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
    pub fn publish(&mut self, peer_id: PeerId, line: String) {
        println!("publish: {}", line);
        self.events.push_front(RosEvent::Publish { peer_id });
    }
}

// impl UpgradeInfo for Ros {
//     type Info = &'static [u8];
//     type InfoIter = iter::Once<Self::Info>;
//
//     fn protocol_info(&self) -> Self::InfoIter {
//         iter::once(PROTOCOL_NAME)
//     }
// }

/// Event that can be emitted by the ROS behaviour.
#[derive(Debug)]
pub enum RosEvent {
    Publish { peer_id: PeerId },
}

impl NetworkBehaviour for Ros {
    type ConnectionHandler = RosHandler;
    type OutEvent = RosEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        RosHandler::new()
    }

    fn inject_event(&mut self, peer_id: PeerId, _: ConnectionId, handler_event: RosHandlerEvent) {
        println!("Event!1-------------- {:?}", handler_event);
        self.events.push_front(RosEvent::Publish { peer_id })
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
            "1Connection established {:?} {:?} {:?}",
            peer_id, connection_id, endpoint
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
            "1Connection closed {:?} {:?} {:?}",
            peer_id, connection_id, endpoint
        );
    }

    fn poll(
        &mut self,
        _: &mut Context<'_>,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        if let Some(e) = self.events.pop_back() {
            let RosEvent::Publish { peer_id } = &e;
            Poll::Ready(NetworkBehaviourAction::GenerateEvent(e))
        } else {
            Poll::Pending
        }
    }
}

enum InboundSubstreamState {
    WaitingInput(libp2p::swarm::NegotiatedSubstream),
    Ready,
    NotReady,
    /// An error occurred during processing.
    Poisoned,
}

enum OutboundSubstreamState {
    WaitingOutput(libp2p::swarm::NegotiatedSubstream),
    Ready,
    NotReady,
    /// An error occurred during processing.
    Poisoned,
}

/// Protocol Handler that manages a single long-lived substream with a peer.
pub struct RosHandler {
    /// Flag indicating that an outbound substream is being established
    /// to prevent duplicate requests.
    outbound_substream_establishing: bool,

    /// The number of outbound substreams we have created.
    outbound_substreams_created: usize,

    inbound_substreams_created: usize,

    events: VecDeque<RosHandlerEvent>,

    /// The single long-lived outbound substream.
    outbound_substream: Option<OutboundSubstreamState>,

    /// The single long-lived inbound substream.
    inbound_substream: Option<InboundSubstreamState>,
}

impl RosHandler {
    pub fn new() -> Self {
        Self {
            outbound_substream_establishing: false,
            outbound_substreams_created: 0,
            inbound_substreams_created: 0,
            events: VecDeque::new(),
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
}

/// A message sent from the behaviour to the handler.
#[derive(Debug, Clone)]
pub enum RosHandlerIn {
    Publish,
}

impl ConnectionHandler for RosHandler {
    type InEvent = RosHandlerIn;
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
        println!("negotiated inbound");

        self.inbound_substreams_created += 1;

        // new inbound substream. Replace the current one, if it exists.
        println!("New inbound substream request");
        self.inbound_substream = Some(InboundSubstreamState::WaitingInput(substream));
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        substream: NegotiatedSubstream,
        _: Self::OutboundOpenInfo,
    ) {
        println!("negotiated outbound");

        self.outbound_substream_establishing = false;
        self.outbound_substreams_created += 1;

        // Should never establish a new outbound substream if one already exists.
        // If this happens, an outbound message is not sent.
        if self.outbound_substream.is_some() {
            println!("Established an outbound substream with one already available");
            // Add the message back to the send queue
            // self.send_queue.push(message);
        } else {
            self.outbound_substream = Some(OutboundSubstreamState::WaitingOutput(substream));
        }
    }

    fn inject_event(&mut self, a: RosHandlerIn) {
        println!("Event4! {:?}", a);
        self.events.push_front(RosHandlerEvent::Publish);
        // self.events
        //     .push(ConnectionHandlerEvent::OutboundSubstreamRequest {
        //         protocol: SubstreamProtocol::new(ReadyUpgrade::new(PROTOCOL_NAME), ()),
        //     });
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
        use libp2p::core::upgrade::UpgradeError;

        let err = err.map_upgrade_err(|e| {
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
            // Too many inbound substreams have been created, end the connection.
            return Poll::Ready(ConnectionHandlerEvent::Close(crate::error::Error::Other(
                "Max inbound substreams".to_string(),
            )));
        }

        // determine if we need to create the stream
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

        loop {
            match std::mem::replace(
                &mut self.inbound_substream,
                Some(InboundSubstreamState::Poisoned),
            ) {
                Some(InboundSubstreamState::WaitingInput(mut substream)) => {
                    //---------------------------
                }
                _ => {
                    self.inbound_substream = None;
                    break;
                }
            }
        }

        Poll::Pending

        // if let Some(e) = self.events.pop_back() {
        //     println!("+++++++++++++ {:?}", e);
        //     Poll::Ready(ConnectionHandlerEvent::Custom(e))
        // } else {
        //     Poll::Pending
        // }
    }
}
