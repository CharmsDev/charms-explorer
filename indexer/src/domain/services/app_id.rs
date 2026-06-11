//! Charm app_id helpers.

/// Convert a token app_id (`t/HASH/VK`) into the matching NFT app_id (`n/HASH/VK`).
///
/// Non-token app_ids are returned unchanged.
pub fn token_to_nft(app_id: &str) -> String {
    app_id.replacen("t/", "n/", 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_token_prefix() {
        assert_eq!(token_to_nft("t/abc/def"), "n/abc/def");
    }

    #[test]
    fn leaves_nft_app_id_alone() {
        assert_eq!(token_to_nft("n/abc/def"), "n/abc/def");
    }

    #[test]
    fn leaves_contract_app_id_alone() {
        assert_eq!(token_to_nft("c/abc/def"), "c/abc/def");
    }

    #[test]
    fn only_replaces_first_occurrence() {
        assert_eq!(token_to_nft("t/t/x"), "n/t/x");
    }
}
