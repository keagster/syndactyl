# Syndactyl Architecture Diagrams

## Software Architecture

This diagram shows the internal component structure of a single Syndactyl node.

```mermaid
graph TB
    subgraph "Main Application"
        Main[main.rs<br/>Application Entry Point]
    end
    
    subgraph "Core Module"
        Config[Config<br/>config.rs<br/>Load & Parse JSON]
        Observer[Observer<br/>observer.rs<br/>File System Watcher]
        Models[Models<br/>models.rs<br/>Data Structures]
        FileHandler[File Handler<br/>file_handler.rs<br/>File I/O Operations]
    end
    
    subgraph "Network Module"
        Manager[Network Manager<br/>manager.rs<br/>Event Orchestration]
        P2P[Syndactyl P2P<br/>syndactyl_p2p.rs<br/>libp2p Node]
        Behaviour[Syndactyl Behaviour<br/>syndactyl_behaviour.rs<br/>Protocol Composition]
        Transfer[Transfer Tracker<br/>transfer.rs<br/>Chunk Management]
    end
    
    subgraph "libp2p Protocols"
        Gossipsub[Gossipsub<br/>Pub/Sub Messaging]
        Kademlia[Kademlia DHT<br/>Peer Discovery]
        ReqResp[Request-Response<br/>File Transfer]
    end
    
    subgraph "Transport Layer"
        TCP[TCP Transport]
        Noise[Noise Encryption]
        Yamux[Yamux Multiplexing]
    end
    
    subgraph "File System"
        WatchedDir1[Watched Directory 1]
        WatchedDir2[Watched Directory 2]
        WatchedDirN[Watched Directory N...]
    end
    
    subgraph "Network Peers"
        Peer1[Peer 1]
        Peer2[Peer 2]
        PeerN[Peer N...]
    end
    
    %% Main flow
    Main --> Config
    Main --> Observer
    Main --> Manager
    
    %% Config connections
    Config -.->|Configuration Data| Observer
    Config -.->|Configuration Data| Manager
    
    %% Observer connections
    Observer -->|File Events<br/>std::mpsc| Manager
    WatchedDir1 -.->|File Change| Observer
    WatchedDir2 -.->|File Change| Observer
    WatchedDirN -.->|File Change| Observer
    Observer --> FileHandler
    
    %% Manager connections
    Manager --> P2P
    Manager --> Transfer
    Manager --> FileHandler
    Manager -.->|Uses| Models
    
    %% P2P connections
    P2P --> Behaviour
    P2P -->|tokio::mpsc| Manager
    
    %% Behaviour connections
    Behaviour --> Gossipsub
    Behaviour --> Kademlia
    Behaviour --> ReqResp
    
    %% Protocol to transport
    Gossipsub --> TCP
    Kademlia --> TCP
    ReqResp --> TCP
    
    %% Transport stack
    TCP --> Noise
    Noise --> Yamux
    
    %% Network connections
    Yamux <-->|Encrypted P2P| Peer1
    Yamux <-->|Encrypted P2P| Peer2
    Yamux <-->|Encrypted P2P| PeerN
    
    %% Styling
    classDef coreClass fill:#e1f5ff,stroke:#01579b,stroke-width:2px
    classDef networkClass fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef protocolClass fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef transportClass fill:#e8f5e9,stroke:#1b5e20,stroke-width:2px
    classDef externalClass fill:#fce4ec,stroke:#880e4f,stroke-width:2px
    
    class Config,Observer,Models,FileHandler coreClass
    class Manager,P2P,Behaviour,Transfer networkClass
    class Gossipsub,Kademlia,ReqResp protocolClass
    class TCP,Noise,Yamux transportClass
    class WatchedDir1,WatchedDir2,WatchedDirN,Peer1,Peer2,PeerN externalClass
```

## Component Interaction Flow

This diagram shows how a file change propagates through the system.

```mermaid
sequenceDiagram
    participant FS as File System
    participant Obs as Observer Thread
    participant FH as File Handler
    participant Mgr as Network Manager
    participant P2P as P2P Node
    participant GS as Gossipsub
    participant Peer as Remote Peer
    participant RR as Request-Response
    
    Note over FS,RR: File Change Detection & Broadcast
    
    FS->>Obs: File Modified Event
    Obs->>FH: Calculate Hash
    FH-->>Obs: SHA-256 Hash
    Obs->>Obs: Build FileEventMessage
    Obs->>Mgr: Send Event (std::mpsc)
    Mgr->>P2P: Publish to Gossipsub
    P2P->>GS: Broadcast Message
    GS-->>Peer: FileEventMessage
    
    Note over Peer: Peer receives event and<br/>decides file is needed
    
    Peer->>RR: FileTransferRequest
    RR-->>P2P: Request Received
    P2P->>Mgr: Request Event (tokio::mpsc)
    Mgr->>FH: Read File Chunk (0, 1MB)
    FH-->>Mgr: First Chunk Data
    Mgr->>P2P: FileTransferResponse
    P2P->>RR: Send Response
    RR-->>Peer: First Chunk
    
    Note over Peer: Peer requests additional chunks
    
    loop For each remaining chunk
        Peer->>RR: FileChunkRequest (offset)
        RR-->>P2P: Chunk Request
        P2P->>Mgr: Chunk Request Event
        Mgr->>FH: Read Chunk (offset, 1MB)
        FH-->>Mgr: Chunk Data
        Mgr->>P2P: FileTransferResponse
        P2P->>RR: Send Chunk
        RR-->>Peer: Chunk Data
    end
    
    Note over Peer: Peer assembles file,<br/>verifies hash, writes to disk
```

## Data Flow Architecture

This diagram shows how data flows through different layers.

```mermaid
graph LR
    subgraph "Application Layer"
        A1[Observer Events]
        A2[Transfer Tracker]
        A3[File Operations]
    end
    
    subgraph "Protocol Layer"
        P1[Gossipsub<br/>Pub/Sub]
        P2[Kademlia<br/>DHT]
        P3[Request-Response<br/>CBOR]
    end
    
    subgraph "Transport Layer"
        T1[TCP/IP]
        T2[Noise Encryption]
        T3[Yamux Multiplexing]
    end
    
    subgraph "Physical Layer"
        N1[Network Interface]
    end
    
    A1 --> P1
    A2 --> P3
    A3 --> P3
    
    P1 --> T1
    P2 --> T1
    P3 --> T1
    
    T1 --> T2
    T2 --> T3
    T3 --> N1
    
    N1 <--> Internet[Internet/LAN]
    
    classDef appLayer fill:#e3f2fd,stroke:#1565c0,stroke-width:2px
    classDef protocolLayer fill:#fff3e0,stroke:#ef6c00,stroke-width:2px
    classDef transportLayer fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    classDef physicalLayer fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    
    class A1,A2,A3 appLayer
    class P1,P2,P3 protocolLayer
    class T1,T2,T3 transportLayer
    class N1,Internet physicalLayer
```

## Thread Architecture

This diagram shows the threading model.

```mermaid
graph TB
    subgraph "Main Thread (Tokio Runtime)"
        MainAsync[Main Async Context]
        NetMgr[Network Manager Event Loop]
    end
    
    subgraph "Observer Threads (OS Threads)"
        Obs1[Observer Thread 1<br/>std::thread]
        Obs2[Observer Thread 2<br/>std::thread]
        ObsN[Observer Thread N<br/>std::thread]
    end
    
    subgraph "Channels"
        StdMpsc[std::mpsc::channel<br/>Observer Events]
        TokioMpsc[tokio::mpsc::channel<br/>P2P Events]
        Bridge[Bridge Thread<br/>std::mpsc → tokio::mpsc]
    end
    
    Obs1 -->|Send Events| StdMpsc
    Obs2 -->|Send Events| StdMpsc
    ObsN -->|Send Events| StdMpsc
    
    StdMpsc --> Bridge
    Bridge --> TokioMpsc
    
    TokioMpsc --> NetMgr
    
    MainAsync -.->|Spawns| Obs1
    MainAsync -.->|Spawns| Obs2
    MainAsync -.->|Spawns| ObsN
    MainAsync -.->|Spawns| NetMgr
    
    NetMgr -->|tokio::select!| P2PEvents[P2P Events]
    NetMgr -->|tokio::select!| ObsEvents[Observer Events]
    NetMgr -->|tokio::select!| SwarmEvents[Swarm Events]
    
    classDef asyncClass fill:#e1f5ff,stroke:#0277bd,stroke-width:2px
    classDef threadClass fill:#fff9c4,stroke:#f57f17,stroke-width:2px
    classDef channelClass fill:#f3e5f5,stroke:#6a1b9a,stroke-width:2px
    
    class MainAsync,NetMgr,P2PEvents,ObsEvents,SwarmEvents asyncClass
    class Obs1,Obs2,ObsN,Bridge threadClass
    class StdMpsc,TokioMpsc channelClass
```

---

## Viewing These Diagrams

These diagrams use Mermaid.js syntax and can be viewed in:

1. **GitHub** - Natively renders Mermaid in markdown files
2. **GitLab** - Natively supports Mermaid
3. **VS Code** - Install "Markdown Preview Mermaid Support" extension
4. **Online** - [Mermaid Live Editor](https://mermaid.live/)
5. **Obsidian** - Natively supports Mermaid
6. **Notion** - Via Mermaid blocks

### Rendering Locally

If you have Node.js installed, you can render to SVG/PNG:

```bash
npm install -g @mermaid-js/mermaid-cli
mmdc -i architecture-diagram.md -o architecture-diagram.svg
```
