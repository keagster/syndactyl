use std::{error::Error, time::Duration};
use futures::prelude::*;
use libp2p::{noise, ping, request_response::{RequestResponse, RequestResponseConfig}, swarm::SwarmEvent, tcp, yamux, Multiaddr, Swarm};
use libp2p::mdns::Mdns;
use crate::network::behaviour::{Behaviour, SyndactylCodec, SyndactylProtocol};
use tracing::{info, debug, warn, error, instrument, span, Level};
use tracing_subscriber::EnvFilter;
use tokio::sync::oneshot;
use metrics::{counter, describe_counter};
use metrics_exporter_prometheus::PrometheusBuilder;

/// Build and configure the libp2p Swarm.
#[instrument(level = "info")]
fn build_swarm() -> Result<Swarm<Behaviour>, Box<dyn Error>> {
    let identity = libp2p::identity::Keypair::generate_ed25519();
    let transport = libp2p::tokio_development_transport(identity.clone())?;

    // Ping behaviour
    let ping = ping::Behaviour::default();

    // RequestResponse behaviour for Syndactyl
    let protocol = SyndactylProtocol();
    let codec = SyndactylCodec::default();
    let mut cfg = RequestResponseConfig::default();
    cfg.set_connection_keep_alive(Duration::from_secs(60));
    let syndactyl = RequestResponse::new(codec, std::iter::once((protocol, cfg)), Default::default());

    // mDNS for peer discovery
    let mdns = tokio::runtime::Handle::current().block_on(Mdns::new(Default::default()))?;

    let behaviour = Behaviour {
        ping,
        syndactyl,
        mdns,
    };

    let swarm = Swarm::with_tokio_executor(transport, behaviour, identity.public().to_peer_id());
    Ok(swarm)
}

/// Run the swarm event loop until shutdown signal is received.
#[instrument(level = "info", skip(swarm, shutdown_rx))]
async fn run_event_loop(
    mut swarm: Swarm<Behaviour>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(target: "libp2p", "Listening on address: {address:?}");
                        counter!("libp2p_listen_addrs").increment(1);
                    }
                    SwarmEvent::Behaviour(event) => {
                        debug!(target: "libp2p", "Behaviour event: {event:?}");
                        counter!("libp2p_behaviour_events").increment(1);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        if let Some(err) = cause {
                            warn!(target: "libp2p", "Connection to {peer_id} closed with error: {err}");
                            counter!("libp2p_connection_errors").increment(1);
                        } else {
                            info!(target: "libp2p", "Connection to {peer_id} closed cleanly");
                            counter!("libp2p_connection_closed").increment(1);
                        }
                    }
                    SwarmEvent::IncomingConnectionError { send_back_addr, error, .. } => {
                        error!(target: "libp2p", "Failed incoming connection from {send_back_addr:?}: {error}");
                        counter!("libp2p_incoming_connection_errors").increment(1);
                    }
                    _ => {}
                }
            }
            _ = &mut shutdown_rx => {
                info!("Shutdown signal received, stopping event loop.");
                break;
            }
        }
    }
}

#[tokio::main]
async fn peer2peer() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .ok();

    // Describe metrics for documentation in Prometheus
    describe_counter!("libp2p_listen_addrs", "Number of listen addresses announced");
    describe_counter!("libp2p_behaviour_events", "Number of behaviour events observed");
    describe_counter!("libp2p_connection_errors", "Number of connection errors");
    describe_counter!("libp2p_connection_closed", "Number of clean connection closures");
    describe_counter!("libp2p_incoming_connection_errors", "Number of incoming connection errors");

    // Start Prometheus exporter on 127.0.0.1:9000/metrics
    PrometheusBuilder::new()
        .with_http_listener(([127, 0, 0, 1], 9000))
        .install()
        .expect("failed to install Prometheus recorder");

    let mut swarm = build_swarm()?;

    swarm.listen_on("/ip4/0.0.0.0/tcp/49999".parse()?)?;

    if let Some(addr) = std::env::args().nth(1) {
        // Create a span for dialing a peer for better context in logs
        let dial_span = span!(Level::INFO, "dial_peer", remote_addr = %addr);
        let _enter = dial_span.enter();

        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        info!("Dialed remote address");
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let event_loop = tokio::spawn(run_event_loop(swarm, shutdown_rx));

    tokio::signal::ctrl_c().await?;
    let _ = shutdown_tx.send(());
    let _ = event_loop.await;

    Ok(())
}
