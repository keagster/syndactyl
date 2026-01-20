# Syndactyl Network Topology Examples

## Example Network: 20 Nodes with 2 Shared Directories

This example shows a realistic deployment with 20 systems sharing two different directories ("ProjectAlpha" and "CompanyDocs").

### Network Overview

```mermaid
graph TB
    subgraph "Bootstrap Nodes"
        B1[Bootstrap Node 1<br/>Alice's Desktop<br/>IP: 192.168.1.10]
        B2[Bootstrap Node 2<br/>Bob's Server<br/>IP: 192.168.1.20]
    end
    
    subgraph "ProjectAlpha Subscribers (12 nodes)"
        PA1[Charlie<br/>Desktop]
        PA2[Diana<br/>Laptop]
        PA3[Eve<br/>Server]
        PA4[Frank<br/>Workstation]
        PA5[Grace<br/>Laptop]
        PA6[Heidi<br/>Desktop]
        PA7[Ivan<br/>Laptop]
        PA8[Judy<br/>Workstation]
        PA9[Mallory<br/>Desktop]
        PA10[Niaj<br/>Laptop]
        PA11[Oscar<br/>Server]
        PA12[Peggy<br/>Desktop]
    end
    
    subgraph "CompanyDocs Subscribers (15 nodes)"
        CD1[Alice<br/>Desktop]
        CD2[Bob<br/>Server]
        CD3[Carol<br/>Laptop]
        CD4[Dave<br/>Desktop]
        CD5[Ellen<br/>Laptop]
        CD6[Fred<br/>Workstation]
        CD7[Gina<br/>Desktop]
        CD8[Henry<br/>Laptop]
        CD9[Iris<br/>Desktop]
        CD10[Jack<br/>Server]
        CD11[Kate<br/>Laptop]
        CD12[Leo<br/>Desktop]
        CD13[Mia<br/>Laptop]
        CD14[Nick<br/>Workstation]
        CD15[Olivia<br/>Desktop]
    end
    
    subgraph "Both Directories (3 nodes)"
        BOTH1[Alice - Desktop<br/>Both Directories]
        BOTH2[Bob - Server<br/>Both Directories]
        BOTH3[Charlie - Desktop<br/>Both Directories]
    end
    
    %% Bootstrap connections
    B1 <-.->|Kademlia DHT| B2
    
    %% All nodes connect to bootstrap for discovery
    PA1 & PA2 & PA3 & PA4 & PA5 & PA6 -.->|Initial Discovery| B1
    PA7 & PA8 & PA9 & PA10 & PA11 & PA12 -.->|Initial Discovery| B2
    
    CD1 & CD2 & CD3 & CD4 & CD5 -.->|Initial Discovery| B1
    CD6 & CD7 & CD8 & CD9 & CD10 -.->|Initial Discovery| B2
    CD11 & CD12 & CD13 & CD14 & CD15 -.->|Initial Discovery| B1
    
    %% Note about topology
    Note1[Note: After initial discovery,<br/>nodes form mesh connections<br/>directly with each other]
    
    classDef bootstrap fill:#ff6b6b,stroke:#c92a2a,stroke-width:3px,color:#fff
    classDef projectAlpha fill:#4ecdc4,stroke:#0a7e7a,stroke-width:2px
    classDef companyDocs fill:#ffe66d,stroke:#c9a700,stroke-width:2px
    classDef both fill:#a8e6cf,stroke:#3d8361,stroke-width:3px
    classDef note fill:#f0f0f0,stroke:#666,stroke-width:1px,stroke-dasharray: 5 5
    
    class B1,B2 bootstrap
    class PA1,PA2,PA3,PA4,PA5,PA6,PA7,PA8,PA9,PA10,PA11,PA12 projectAlpha
    class CD1,CD2,CD3,CD4,CD5,CD6,CD7,CD8,CD9,CD10,CD11,CD12,CD13,CD14,CD15 companyDocs
    class BOTH1,BOTH2,BOTH3 both
    class Note1 note
```

## Gossipsub Topic Topology

This shows how nodes subscribe to different Gossipsub topics based on their directory subscriptions.

```mermaid
graph LR
    subgraph "Gossipsub Topics"
        T1[Topic: syndactyl-gossip<br/>Global Events]
        T2[Topic: projectalpha<br/>Optional Future Feature]
        T3[Topic: companydocs<br/>Optional Future Feature]
    end
    
    subgraph "Current Implementation"
        AllNodes[All 20 Nodes<br/>Subscribe to<br/>syndactyl-gossip]
    end
    
    subgraph "Node Filtering"
        Filter[Nodes filter messages<br/>based on 'observer' field<br/>in FileEventMessage]
    end
    
    AllNodes -->|Subscribe| T1
    T1 -->|Broadcast all events| AllNodes
    AllNodes --> Filter
    
    Filter -->|Keep if observer matches| Process[Process File Event]
    Filter -->|Drop if observer doesn't match| Ignore[Ignore Event]
    
    classDef topic fill:#667eea,stroke:#4c51bf,stroke-width:2px,color:#fff
    classDef node fill:#48bb78,stroke:#2f855a,stroke-width:2px
    classDef logic fill:#ed8936,stroke:#c05621,stroke-width:2px
    
    class T1,T2,T3 topic
    class AllNodes node
    class Filter,Process,Ignore logic
```

## P2P Mesh Network Topology

After initial bootstrap discovery, nodes form direct connections in a mesh topology.

```mermaid
graph TB
    subgraph "Mesh Network - ProjectAlpha Directory"
        N1[Node 1]
        N2[Node 2]
        N3[Node 3]
        N4[Node 4]
        N5[Node 5]
        N6[Node 6]
        N7[Node 7]
        N8[Node 8]
        N9[Node 9]
        N10[Node 10]
        N11[Node 11]
        N12[Node 12]
    end
    
    %% Mesh connections (partial - full mesh would be too complex)
    N1 <-->|Gossipsub| N2
    N1 <-->|Gossipsub| N3
    N1 <-->|Gossipsub| N4
    N2 <-->|Gossipsub| N3
    N2 <-->|Gossipsub| N5
    N3 <-->|Gossipsub| N6
    N4 <-->|Gossipsub| N7
    N5 <-->|Gossipsub| N8
    N6 <-->|Gossipsub| N9
    N7 <-->|Gossipsub| N10
    N8 <-->|Gossipsub| N11
    N9 <-->|Gossipsub| N12
    N10 <-->|Gossipsub| N11
    N11 <-->|Gossipsub| N12
    N4 <-->|Gossipsub| N8
    N5 <-->|Gossipsub| N9
    N6 <-->|Gossipsub| N10
    
    %% File transfer connections (Request-Response)
    N1 -.->|File Transfer| N7
    N3 -.->|File Transfer| N11
    N5 -.->|File Transfer| N2
    
    Note2[Note: Gossipsub manages mesh<br/>topology automatically with<br/>configurable degree parameter]
    
    classDef meshNode fill:#3b82f6,stroke:#1e40af,stroke-width:2px,color:#fff
    classDef note fill:#f0f0f0,stroke:#666,stroke-width:1px,stroke-dasharray: 5 5
    
    class N1,N2,N3,N4,N5,N6,N7,N8,N9,N10,N11,N12 meshNode
    class Note2 note
```

## File Synchronization Flow Example

This shows what happens when a file is modified on one node and synced to others.

```mermaid
sequenceDiagram
    participant N1 as Node 1<br/>(Alice)
    participant GS as Gossipsub Network
    participant N2 as Node 2<br/>(Bob)
    participant N3 as Node 3<br/>(Charlie)
    participant N4 as Node 4<br/>(Diana)
    
    Note over N1,N4: All nodes watching "ProjectAlpha" directory
    
    rect rgb(200, 230, 255)
    Note right of N1: Alice modifies report.pdf
    N1->>N1: Detect file change
    N1->>N1: Calculate hash: abc123...
    end
    
    rect rgb(255, 230, 200)
    Note over GS: Broadcast Phase
    N1->>GS: Publish FileEventMessage<br/>{observer: "ProjectAlpha",<br/>path: "report.pdf",<br/>hash: "abc123...",<br/>size: 2048576}
    GS->>N2: FileEventMessage
    GS->>N3: FileEventMessage
    GS->>N4: FileEventMessage
    end
    
    rect rgb(200, 255, 200)
    Note over N2,N4: Decision Phase
    N2->>N2: Check local file<br/>hash: xyz789...<br/>DIFFERENT - Need update
    N3->>N3: Check local file<br/>hash: abc123...<br/>SAME - Skip
    N4->>N4: File doesn't exist<br/>Need download
    end
    
    rect rgb(255, 200, 255)
    Note over N2,N4: Transfer Phase
    N2->>N1: FileTransferRequest
    N1->>N2: FileTransferResponse<br/>Chunk 1 (1MB)
    N2->>N1: FileChunkRequest (offset: 1MB)
    N1->>N2: FileTransferResponse<br/>Chunk 2 (0.95MB, last)
    N2->>N2: Verify hash & write to disk
    
    par Node 4 downloads in parallel
        N4->>N1: FileTransferRequest
        N1->>N4: FileTransferResponse<br/>Chunk 1 (1MB)
        N4->>N1: FileChunkRequest (offset: 1MB)
        N1->>N4: FileTransferResponse<br/>Chunk 2 (0.95MB, last)
        N4->>N4: Verify hash & write to disk
    end
    end
    
    Note over N1,N4: Synchronization Complete
```

## Physical Network Layout Example

```mermaid
graph TB
    subgraph "Office Network - 192.168.1.0/24"
        Router1[Office Router<br/>192.168.1.1]
        Desktop1[Desktop 1<br/>192.168.1.10<br/>Bootstrap Node]
        Desktop2[Desktop 2<br/>192.168.1.11]
        Desktop3[Desktop 3<br/>192.168.1.12]
        Server1[File Server<br/>192.168.1.20<br/>Bootstrap Node]
        Laptop1[Laptop 1<br/>192.168.1.30]
        Laptop2[Laptop 2<br/>192.168.1.31]
    end
    
    subgraph "Remote Office - 10.0.0.0/24"
        Router2[Remote Router<br/>10.0.0.1]
        Desktop4[Desktop 4<br/>10.0.0.10]
        Desktop5[Desktop 5<br/>10.0.0.11]
        Laptop3[Laptop 3<br/>10.0.0.20]
    end
    
    subgraph "Home Workers"
        Home1[Home Desktop<br/>Dynamic IP<br/>NAT Traversal]
        Home2[Home Laptop<br/>Dynamic IP<br/>NAT Traversal]
    end
    
    subgraph "Cloud VPS"
        VPS1[Cloud Server<br/>Public IP<br/>Bootstrap Node]
    end
    
    Internet((Internet))
    
    Router1 <--> Internet
    Router2 <--> Internet
    Home1 <--> Internet
    Home2 <--> Internet
    VPS1 <--> Internet
    
    Desktop1 & Desktop2 & Desktop3 & Server1 & Laptop1 & Laptop2 --> Router1
    Desktop4 & Desktop5 & Laptop3 --> Router2
    
    Note3[Note: Kademlia DHT helps with<br/>NAT traversal and peer discovery<br/>across different networks]
    
    classDef router fill:#ff6b6b,stroke:#c92a2a,stroke-width:2px,color:#fff
    classDef bootstrap fill:#51cf66,stroke:#2f9e44,stroke-width:3px
    classDef office fill:#74c0fc,stroke:#1971c2,stroke-width:2px
    classDef remote fill:#ffd43b,stroke:#f59f00,stroke-width:2px
    classDef home fill:#da77f2,stroke:#9c36b5,stroke-width:2px
    classDef cloud fill:#ff8787,stroke:#e03131,stroke-width:2px
    classDef internet fill:#495057,stroke:#212529,stroke-width:3px,color:#fff
    classDef note fill:#f0f0f0,stroke:#666,stroke-width:1px,stroke-dasharray: 5 5
    
    class Router1,Router2 router
    class Desktop1,Server1,VPS1 bootstrap
    class Desktop2,Desktop3,Laptop1,Laptop2 office
    class Desktop4,Desktop5,Laptop3 remote
    class Home1,Home2 home
    class VPS1 cloud
    class Internet internet
    class Note3 note
```

## Configuration Example for Multi-Directory Setup

Here's how a node would be configured to participate in both directories:

```json
{
  "observers": [
    {
      "name": "ProjectAlpha",
      "path": "/home/user/sync/ProjectAlpha"
    },
    {
      "name": "CompanyDocs",
      "path": "/home/user/sync/CompanyDocs"
    }
  ],
  "network": {
    "listen_addr": "0.0.0.0",
    "port": "49999",
    "dht_mode": "server",
    "bootstrap_peers": [
      {
        "ip": "192.168.1.10",
        "port": "49999",
        "peer_id": "12D3KooWAbc123...BootstrapNode1"
      },
      {
        "ip": "192.168.1.20",
        "port": "49999",
        "peer_id": "12D3KooWDef456...BootstrapNode2"
      },
      {
        "ip": "203.0.113.50",
        "port": "49999",
        "peer_id": "12D3KooWGhi789...CloudBootstrap"
      }
    ]
  }
}
```

## Directory Access Matrix

This table shows which nodes have access to which directories in our example:

| Node | ProjectAlpha | CompanyDocs | Role |
|------|--------------|-------------|------|
| Alice | ✓ | ✓ | Bootstrap |
| Bob | ✓ | ✓ | Bootstrap |
| Charlie | ✓ | ✓ | Regular |
| Diana | ✓ | ✗ | Regular |
| Eve | ✓ | ✗ | Regular |
| Frank | ✓ | ✗ | Regular |
| Grace | ✓ | ✗ | Regular |
| Heidi | ✓ | ✗ | Regular |
| Ivan | ✓ | ✗ | Regular |
| Judy | ✓ | ✗ | Regular |
| Mallory | ✓ | ✗ | Regular |
| Niaj | ✓ | ✗ | Regular |
| Oscar | ✓ | ✗ | Regular |
| Peggy | ✓ | ✗ | Regular |
| Carol | ✗ | ✓ | Regular |
| Dave | ✗ | ✓ | Regular |
| Ellen | ✗ | ✓ | Regular |
| Fred | ✗ | ✓ | Regular |
| Gina | ✗ | ✓ | Regular |
| Henry | ✗ | ✓ | Regular |
| Iris | ✗ | ✓ | Regular |
| Jack | ✗ | ✓ | Regular |

**Total: 20 unique nodes**
- ProjectAlpha subscribers: 15 nodes (12 exclusive + 3 both)
- CompanyDocs subscribers: 15 nodes (12 exclusive + 3 both)
- Bootstrap nodes: 2-3 (should have good uptime)

## Network Statistics for This Setup

```mermaid
pie title Directory Subscriptions
    "ProjectAlpha Only" : 12
    "CompanyDocs Only" : 12
    "Both Directories" : 3
```

```mermaid
pie title Node Types
    "Regular Nodes" : 17
    "Bootstrap Nodes" : 3
```

## Scalability Considerations

```mermaid
graph LR
    subgraph "Network Growth"
        S1[20 Nodes] -->|Add Users| S2[50 Nodes]
        S2 -->|Add Departments| S3[100 Nodes]
        S3 -->|Enterprise Scale| S4[500+ Nodes]
    end
    
    subgraph "Performance Factors"
        F1[Gossipsub Mesh Degree<br/>Default: 6-12 connections]
        F2[File Transfer<br/>1MB chunks<br/>Direct P2P]
        F3[Kademlia DHT<br/>Log N discovery time]
    end
    
    subgraph "Recommended Limits"
        L1[Observers per Node<br/>< 10 recommended]
        L2[File Size<br/>< 10GB per file]
        L3[Directory Size<br/>Monitor < 100K files]
    end
    
    S1 -.-> F1
    S2 -.-> F2
    S3 -.-> F3
    S4 -.-> L1
    S4 -.-> L2
    S4 -.-> L3
    
    classDef scale fill:#4ecdc4,stroke:#0a7e7a,stroke-width:2px
    classDef perf fill:#ffe66d,stroke:#c9a700,stroke-width:2px
    classDef limit fill:#ff6b6b,stroke:#c92a2a,stroke-width:2px
    
    class S1,S2,S3,S4 scale
    class F1,F2,F3 perf
    class L1,L2,L3 limit
```

---

## Key Takeaways

1. **Decentralized**: No central server required; any node can go offline without affecting others
2. **Mesh Topology**: Nodes connect directly to each other after bootstrap discovery
3. **Selective Sync**: Nodes only sync directories they're configured for
4. **Efficient Broadcasting**: Gossipsub prevents message duplication and manages mesh connections
5. **Direct Transfers**: Large files are transferred directly peer-to-peer, not broadcasted
6. **Bootstrap Nodes**: Help with initial discovery but aren't required after connections are established
7. **NAT Traversal**: Kademlia DHT helps nodes behind NAT connect to each other
