use anyhow::Result;
use bitcoincore_rpc::bitcoin::consensus::deserialize;
use bitcoincore_rpc::bitcoin::{Address, Network, Transaction};

/// Extracts Bitcoin addresses from transaction hex data
pub struct AddressExtractor;

impl AddressExtractor {
    /// Extract all output addresses from a transaction hex string
    fn extract_all_addresses(tx_hex: &str, network: &str) -> Result<Vec<String>> {
        let tx_bytes = hex::decode(tx_hex)?;
        let tx: Transaction = deserialize(&tx_bytes)?;
        
        let btc_network = match network {
            "mainnet" => Network::Bitcoin,
            "testnet4" => Network::Testnet,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            _ => Network::Testnet,
        };
        
        let mut addresses = Vec::new();
        
        for output in &tx.output {
            if let Ok(address) = Address::from_script(&output.script_pubkey, btc_network) {
                addresses.push(address.to_string());
            }
        }
        
        Ok(addresses)
    }
    
    /// Extract the address that likely holds the charm asset.
    /// Prioritizes P2PKH and P2SH over bech32 segwit/taproot.
    pub fn extract_charm_holder_address(tx_hex: &str, network: &str) -> Result<Option<String>> {
        let addresses = Self::extract_all_addresses(tx_hex, network)?;

        let preferred = addresses
            .iter()
            .find(|a| a.starts_with('1') || a.starts_with('m') || a.starts_with('n'))
            .or_else(|| {
                addresses
                    .iter()
                    .find(|a| a.starts_with('3') || a.starts_with('2'))
            })
            .or_else(|| {
                addresses
                    .iter()
                    .find(|a| a.starts_with("bc1") || a.starts_with("tb1"))
            });

        Ok(preferred.cloned().or_else(|| addresses.first().cloned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real V10 charm tx (4 outputs: 3 P2WPKH bech32 mainnet, 1 OP_RETURN).
    /// Reused from native_charm_parser regression tests.
    const V10_CHARM_TX_HEX: &str = "02000000000101014eec217f37aa11bf55461745803c3a41fb0796346dc747f2b6a98a6e5ab6cd0300000000ffffffff043c280000000000001600144344ab076e827b487b1f865892d27501eabcc05a770d000000000000160014318d2dbf53a3f9c41b2e36683a3a8b8580e055160000000000000000fd22046a057370656c6c4d180482a36776657273696f6e0a627478a1646f75747381a100a7656d616b6572783e6263317063327538776d3874716a6865306c39616a746868646661307368617973307667346b32676e3561753977616c6d64787176677373666e6e30787469657865635f74797065a1677061727469616ca0647369646563626964657072696365821913880166616d6f756e74192710687175616e7469747902656173736574a165746f6b656e7883742f336437666537653463656136313231393437616637336437306535313139626562643861613562376564666537346266616636653737396131383437626439622f63393735643465306332393266623935656662646135633133333132643661633164386235616566663766306631653535373836343561326461373066663566716170705f7075626c69635f696e70757473a283616298200000000000000000000000000000000000000000000000000000000000000000982018a4187118d318fc18c4183618ae187c18bc0e0c188218a6188c18dc188e00183e18e2181e18f8181918e118ac18f8183418e1181c184318ce184718d8f68361749820183d187f18e718e418ce18a6121819184718af187318d70e1851181918be18bd188a18a518b718ed18fe187418bf18af186e1877189a1818184718bd189b982018c9187518d418e018c2189218fb189518ef18bd18a518c118331218d618ac181d188b185a18ef18f718f018f118e518571886184518a218da187018ff185ff699010418a41859184c1859182f18cb18c805181a188412161862188a184504181c189a18c51824187a0318e41871185218ef18e21819181818ed1850188718d118a118221832151841185c186818be18fa18d00818241883183d181c18bf18dd1866182317184e1823183e18e618b41858182a1896182818b401184918f618971852182d18781888185a181e18b1185218ed18c2184b1824187d18db18501859189318ae187718221871182d183418fe1827187118e11886181d1824183f185d1821181918f618d218b51851184b185418c01889181c18be188e061871187d18f418bd18e4187418c718a31418421889188c187118a718d318c618f3182b1894182418cb184f11184e1218bd18e618ec18e21867187918a9188c184a18ed18380518fd18da188818eb18361824189118a7181918ec188518e01884081718c918aa051888187318f51854186118801518461418ad0818e1183d18af18d5186d186218d018d018ea18b5189c186818c518440f18be18e00318de186e184118a118c118bc1857183818a1187a184318ce18df184f1829185712187118851853183418ce1318ce181b186f18f2189518ef188f18a91418a9187b182818c218c1187918e71850188918c718ec183e18571868186d18fe189618450818cc18b718cc188d185c189a18f6187d0518a518870bd511200000000000225120c2b8776ceb04af97fcbd92ef76a7af85fa483d88ad9489d3bc2bbbfdb4c0622101406da3eb0e8b2e86d3af844eca8813670a68891edb8e4cc239ebaad96085345928666f0b5d3c5b42e451dc884748f687802b3eb4c70d3d53c7fa67552fcfd06f5e00000000";

    #[test]
    fn invalid_hex_returns_error() {
        let result = AddressExtractor::extract_charm_holder_address("zzzz", "mainnet");
        assert!(result.is_err());
    }

    #[test]
    fn truncated_tx_bytes_return_error() {
        let result = AddressExtractor::extract_charm_holder_address("deadbeef", "mainnet");
        assert!(result.is_err());
    }

    #[test]
    fn extract_all_addresses_mainnet_bech32() {
        let res = AddressExtractor::extract_all_addresses(V10_CHARM_TX_HEX, "mainnet")
            .expect("parse");
        // tx has 3 P2WPKH + 1 OP_RETURN + 1 P2TR change output
        assert!(!res.is_empty(), "expected at least one address");
        for addr in &res {
            assert!(
                addr.starts_with("bc1"),
                "mainnet output should be bech32: {addr}"
            );
        }
    }

    #[test]
    fn unknown_network_falls_back_to_testnet() {
        let res = AddressExtractor::extract_all_addresses(V10_CHARM_TX_HEX, "regtest-foo")
            .expect("parse");
        for addr in &res {
            assert!(addr.starts_with("tb1"), "should be testnet bech32: {addr}");
        }
    }

    #[test]
    fn holder_picks_bech32_when_no_legacy_outputs() {
        let res = AddressExtractor::extract_charm_holder_address(V10_CHARM_TX_HEX, "mainnet")
            .expect("parse")
            .expect("at least one address");
        assert!(res.starts_with("bc1"), "got: {res}");
    }
}

