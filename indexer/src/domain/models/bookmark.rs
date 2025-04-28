/// Domain model for a bookmark that tracks the last processed block
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Block hash
    pub hash: String,

    /// Block height
    pub height: u64,

    /// Status of the block (pending, confirmed, etc.)
    pub status: String,
}

impl Bookmark {
    /// Create a new Bookmark
    pub fn new(hash: String, height: u64, status: String) -> Self {
        Self {
            hash,
            height,
            status,
        }
    }

    /// Create a new pending bookmark
    pub fn pending(hash: String, height: u64) -> Self {
        Self::new(hash, height, "pending".to_string())
    }

    /// Create a new confirmed bookmark
    pub fn confirmed(hash: String, height: u64) -> Self {
        Self::new(hash, height, "confirmed".to_string())
    }

    /// Check if the bookmark is confirmed
    pub fn is_confirmed(&self) -> bool {
        self.status == "confirmed"
    }
}
