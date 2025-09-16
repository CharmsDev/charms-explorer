/// Tracks the last processed block for a blockchain network
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Block hash
    pub hash: String,

    /// Block height
    pub height: u64,

    /// Status of the block (pending, confirmed, etc.)
    pub status: String,

    /// Blockchain type (e.g., "Bitcoin", "Cardano")
    pub blockchain: String,

    /// Network name (e.g., "mainnet", "testnet4")
    pub network: String,
}

impl Bookmark {
    /// Creates a new bookmark with specified parameters
    pub fn new(
        hash: String,
        height: u64,
        status: String,
        blockchain: String,
        network: String,
    ) -> Self {
        Self {
            hash,
            height,
            status,
            blockchain,
            network,
        }
    }

    /// Creates a bookmark with pending status
    pub fn pending(hash: String, height: u64, blockchain: String, network: String) -> Self {
        Self::new(hash, height, "pending".to_string(), blockchain, network)
    }

    /// Creates a bookmark with confirmed status
    pub fn confirmed(hash: String, height: u64, blockchain: String, network: String) -> Self {
        Self::new(hash, height, "confirmed".to_string(), blockchain, network)
    }

    /// Returns true if bookmark has confirmed status
    pub fn is_confirmed(&self) -> bool {
        self.status == "confirmed"
    }
}
