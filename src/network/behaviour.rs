use libp2p::{
    NetworkBehaviour,
    PeerId,
    request_response::{
        ProtocolName, RequestResponse, RequestResponseCodec, RequestResponseConfig,
        RequestResponseEvent, RequestResponseMessage, RequestId, ResponseChannel,
    },
    swarm::NetworkBehaviourEventProcess,
};
use std::io;

// 1. Define protocol name
#[derive(Clone)]
pub struct SyndactylProtocol();
impl ProtocolName for SyndactylProtocol {
    fn protocol_name(&self) -> &[u8] {
        b"/syndactyl/1.0.0"
    }
}

// 2. Define codec
#[derive(Clone, Default)]
pub struct SyndactylCodec();
impl RequestResponseCodec for SyndactylCodec {
    type Protocol = SyndactylProtocol;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn read_request<T>(&mut self, _: &SyndactylProtocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: io::Read,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn read_response<T>(&mut self, _: &SyndactylProtocol, io: &mut T) -> io::Result<Self::Response>
    where
        T: io::Read,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn write_request<T>(&mut self, _: &SyndactylProtocol, io: &mut T, data: Self::Request) -> io::Result<()> 
    where
        T: io::Write,
    {
        io.write_all(&data)
    }

    fn write_response<T>(&mut self, _: &SyndactylProtocol, io: &mut T, data: Self::Response) -> io::Result<()> 
    where
        T: io::Write,
    {
        io.write_all(&data)
    }
}

// 3. Add to your behaviour
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct Behaviour {
    pub ping: libp2p::ping::Ping,
    pub syndactyl: RequestResponse<SyndactylCodec>,
}

#[derive(Debug)]
pub enum OutEvent {
    Ping(libp2p::ping::PingEvent),
    Syndactyl(RequestResponseEvent<Vec<u8>, Vec<u8>>),
}

// Example method to send a request
impl Behaviour {
    pub fn send_syndactyl_request(&mut self, peer: &PeerId, data: Vec<u8>) -> RequestId {
        self.syndactyl.send_request(peer, data)
    }
}

// Handle incoming requests and responses
impl NetworkBehaviourEventProcess<RequestResponseEvent<Vec<u8>, Vec<u8>>> for Behaviour {
    fn inject_event(&mut self, event: RequestResponseEvent<Vec<u8>, Vec<u8>>) {
        match event {
            RequestResponseEvent::Message { peer, message } => match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    // Handle incoming request
                    // For example, echo the request back as the response:
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
