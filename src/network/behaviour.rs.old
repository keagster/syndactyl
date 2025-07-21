use libp2p::{
    core::PeerId,
    mdns::{Behaviour as Mdns, Event as MdnsEvent},
    ping::{Behaviour as Ping, Event as PingEvent},
    request_response::{
        Behaviour as RequestResponseBehaviour,
        Codec as RequestResponseCodec,
        Config as RequestResponseConfig,
        RequestId,
        Event as RequestResponseEvent,
        Message as RequestResponseMessage,
        ProtocolSupport,
    },
    swarm::NetworkBehaviour,
};

#[derive(Clone)]
pub struct SyndactylProtocol();
impl ProtocolName for SyndactylProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/syndactyl/1.0.0"
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(peers) => {
                for (peer_id, _addr) in peers {
                    // Send announce message to discovered peer
                    let announce_msg = b"announce:".to_vec();
                    self.syndactyl.send_request(&peer_id, announce_msg);
                }
            }
            MdnsEvent::Expired(_) => {
                // Handle expired peers if needed
            }
        }
    }
}

// Protocol name as a constant
const SYNDACTYL_PROTOCOL_NAME: &[u8] = b"/syndactyl/1.0.0";

// Codec
#[derive(Clone, Default)]
pub struct SyndactylCodec;

impl RequestResponseCodec for SyndactylCodec {
    type Protocol = std::borrow::Cow<'static, [u8]>;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn decode_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Request>
    where
        T: std::io::Read,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn decode_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> std::io::Result<Self::Response>
    where
        T: std::io::Read,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn encode_request<T>(&mut self, _: &Self::Protocol, io: &mut T, data: Self::Request) -> std::io::Result<()> 
    where
        T: std::io::Write,
    {
        io.write_all(&data)
    }

    fn encode_response<T>(&mut self, _: &Self::Protocol, io: &mut T, data: Self::Response) -> std::io::Result<()> 
    where
        T: std::io::Write,
    {
        io.write_all(&data)
    }
}

// 3. Add to your behaviour
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct Behaviour {
    pub ping: Ping,
    pub syndactyl: RequestResponseBehaviour<SyndactylCodec>,
    pub mdns: Mdns,
}

#[derive(Debug)]
pub enum OutEvent {
    Ping(PingEvent),
    Syndactyl(RequestResponseEvent<Vec<u8>, Vec<u8>>),
    Mdns(MdnsEvent),
}

// Example method to send a request
impl Behaviour {
    pub fn send_syndactyl_request(&mut self, peer: &PeerId, data: Vec<u8>) -> RequestId {
        self.syndactyl.send_request(peer, data)
    }

    // Discover peers (stub for now)
    pub fn discover_peers(&self) {
        // TODO: Implement peer discovery logic
    }

    // Send an action message (e.g., file sync event)
    pub fn send_action_message(&mut self, peer: &PeerId, action: &str, payload: &[u8]) -> RequestId {
        let mut msg = Vec::new();
        msg.extend_from_slice(action.as_bytes());
        msg.push(b':');
        msg.extend_from_slice(payload);
        self.syndactyl.send_request(peer, msg)
    }
}

// Handle incoming requests and responses
impl NetworkBehaviourEventProcess<RequestResponseEvent<Vec<u8>, Vec<u8>>> for Behaviour {
    fn inject_event(&mut self, event: RequestResponseEvent<Vec<u8>, Vec<u8>>) {
        match event {
            RequestResponseEvent::Message { peer, message } => match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    // Handle incoming request
                    // Parse action and payload
                    if let Some(pos) = request.iter().position(|&b| b == b':') {
                        let action = String::from_utf8_lossy(&request[..pos]);
                        let payload = &request[pos+1..];
                        match action.as_ref() {
                            "announce" => {
                                // Handle announce message
                                // TODO: Track/log connected peer
                            }
                            "sync" => {
                                // TODO: Trigger file sync logic with payload
                            }
                            _ => {
                                // Unknown action
                            }
                        }
                    }
                    // Echo the request back as the response:
                    let response = request.clone();
                    self.syndactyl.send_response(channel, response).unwrap_or_else(|_| ());
                }
                RequestResponseMessage::Response { request_id, response } => {
                    // Handle response to a request we sent
                    // e.g., log or process the response
                }
            },
            RequestResponseEvent::OutboundFailure { peer, request_id, error } => {
                // Handle outbound failure
            }
            RequestResponseEvent::InboundFailure { peer, error, .. } => {
                // Handle inbound failure
            }
            RequestResponseEvent::ResponseSent { peer, request_id } => {
                // Optionally handle confirmation that a response was sent
            }
        }
    }
}
