// file: src/network/p2p.rs
use libp2p::{
    swarm::{NetworkBehaviour, Swarm},
    kad::{Kademlia, store::MemoryStore},
    identify, ping, gossipsub,
};

#[derive(NetworkBehaviour)]
pub struct BlockchainBehaviour {
    pub kademlia: Kademlia<MemoryStore>,
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub block_protocol: BlockProtocol,
}

pub struct P2PManager {
    swarm: Swarm<BlockchainBehaviour>,
    peer_manager: PeerManager,
}

impl P2PManager {
    pub async fn new(network: NetworkType) -> Result<Self, NetworkError> {
        // Configure libp2p with quantum-resistant encryption
        let transport = transport_with_quantum_resistance();
        let behaviour = BlockchainBehaviour::new(network);
        let swarm = Swarm::new(transport, behaviour, local_peer_id);

        Ok(Self {
            swarm,
            peer_manager: PeerManager::new(),
        })
    }

    pub async fn broadcast_block(&mut self, block: &Block) -> Result<(), NetworkError> {
        let topic = gossipsub::IdentTopic::new("blocks");
        let message = block.serialize()?;
        self.swarm.behaviour_mut().gossipsub.publish(topic, message)?;
        Ok(())
    }
}