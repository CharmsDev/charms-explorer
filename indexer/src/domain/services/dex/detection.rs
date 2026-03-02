//! DEX detection logic for Charms Cast orders
//!
//! This module analyzes normalized spells to detect DEX operations
//! such as order creation, fulfillment, cancellation, and partial fills.

use serde_json::Value;

use super::types::{DexDetectionResult, DexOperation, DexOrder, ExecType, OrderSide, is_dex_app_id};

/// Detect DEX operations from a charm's JSON data
///
/// Analyzes the spell data to identify DEX-related operations and extract
/// order information when present.
pub fn detect_dex_operation(charm_data: &Value) -> Option<DexDetectionResult> {
    // Get native_data from charm
    let native_data = charm_data.get("native_data")?;

    // Check app_public_inputs for DEX app
    let app_inputs = native_data.get("app_public_inputs")?;
    let dex_app_id = find_dex_app_id(app_inputs)?;

    // Get transaction outputs
    let tx = native_data.get("tx")?;
    let outs = tx.get("outs").and_then(|v| v.as_array());

    // Analyze outputs to determine operation type
    let output_orders = extract_orders_from_outputs(outs, &dex_app_id);

    let operation = determine_operation(&output_orders, outs);

    // Build tags - only product tags, not operation types
    // Operation details go in dex_orders table
    let tags = vec!["charms-cast".to_string()];

    // Extract order details from output if creating/modifying
    let order = output_orders.first().cloned();

    // Input order IDs (empty — spell ins are raw UtxoId bytes, not charm data)
    let input_order_ids: Vec<String> = Vec::new();

    // Build output order ID
    let output_order_id = order.as_ref().and_then(|o| o.scrolls_address.clone());

    Some(DexDetectionResult {
        operation,
        dex_app_id,
        order,
        input_order_ids,
        output_order_id,
        tags,
    })
}

/// Find DEX app_id in app_public_inputs
fn find_dex_app_id(app_inputs: &Value) -> Option<String> {
    // app_public_inputs is an array of [app_id, data] pairs
    if let Some(arr) = app_inputs.as_array() {
        for item in arr {
            if let Some(app_arr) = item.as_array() {
                if let Some(app_id) = app_arr.first().and_then(|v| v.as_str()) {
                    if is_dex_app_id(app_id) {
                        return Some(app_id.to_string());
                    }
                }
            }
        }
    }

    // Also check if it's an object with app_id keys
    if let Some(obj) = app_inputs.as_object() {
        for (app_id, _) in obj {
            if is_dex_app_id(app_id) {
                return Some(app_id.clone());
            }
        }
    }

    None
}

/// Extract order data from transaction outputs
fn extract_orders_from_outputs(outs: Option<&Vec<Value>>, dex_app_id: &str) -> Vec<DexOrder> {
    let mut orders = Vec::new();

    if let Some(outputs) = outs {
        for (idx, output) in outputs.iter().enumerate() {
            if let Some(order) = extract_order_from_output(output, dex_app_id, idx) {
                orders.push(order);
            }
        }
    }

    orders
}

/// Extract order from a charm's output data
fn extract_order_from_output(output: &Value, dex_app_id: &str, _idx: usize) -> Option<DexOrder> {
    // Output structure varies - could be direct charms or nested
    // Try different structures

    // Structure 1: output is a map with app indices as keys
    if let Some(obj) = output.as_object() {
        for (_key, charm_data) in obj {
            if let Some(order) = parse_order_data(charm_data, dex_app_id) {
                return Some(order);
            }
        }
    }

    // Structure 2: output has "charms" field
    if let Some(charms) = output.get("charms") {
        if let Some(obj) = charms.as_object() {
            for (_key, charm_data) in obj {
                if let Some(order) = parse_order_data(charm_data, dex_app_id) {
                    return Some(order);
                }
            }
        }
    }

    None
}

/// Parse order data from charm JSON
fn parse_order_data(data: &Value, _dex_app_id: &str) -> Option<DexOrder> {
    // Check if this looks like an order (has maker, side, price, etc.)
    let maker = data.get("maker").and_then(|v| v.as_str())?;
    let side_str = data.get("side").and_then(|v| v.as_str())?;

    let side = match side_str {
        "ask" => OrderSide::Ask,
        "bid" => OrderSide::Bid,
        _ => return None,
    };

    // Parse exec_type
    let exec_type = parse_exec_type(data.get("exec_type"));

    // Parse price [num, den]
    let price = parse_price(data.get("price"))?;

    // Parse amount and quantity
    let amount = data.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
    let quantity = data.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0);

    // Parse asset
    let asset_app_id = data
        .get("asset")
        .and_then(|a| a.get("token"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(DexOrder {
        maker: maker.to_string(),
        side,
        exec_type,
        price,
        amount,
        quantity,
        asset_app_id,
        scrolls_address: None, // Will be set from output address if available
    })
}

/// Parse exec_type from JSON
fn parse_exec_type(value: Option<&Value>) -> ExecType {
    match value {
        Some(v) if v.is_string() && v.as_str() == Some("all_or_none") => ExecType::AllOrNone,
        Some(v) if v.is_object() => {
            if let Some(partial) = v.get("partial") {
                let from = partial
                    .get("from")
                    .and_then(|f| f.as_str())
                    .map(|s| s.to_string());
                ExecType::Partial { from }
            } else {
                ExecType::AllOrNone
            }
        }
        _ => ExecType::AllOrNone,
    }
}

/// Parse price from JSON [num, den] array
fn parse_price(value: Option<&Value>) -> Option<(u64, u64)> {
    let arr = value?.as_array()?;
    if arr.len() >= 2 {
        let num = arr[0].as_u64()?;
        let den = arr[1].as_u64()?;
        Some((num, den))
    } else {
        None
    }
}

/// Determine operation type based on output orders and spell output count.
///
/// Spell output structure is the primary signal:
///
/// | Operation    | outs_count | has order charm in outs |
/// |------------- |------------|-------------------------|
/// | CREATE-ASK   | 2          | ✓ (side=ask)            |
/// | CREATE-BID   | 1          | ✓ (side=bid)            |
/// | FULFILL-ASK  | 3          | ✗                       |
/// | FULFILL-BID  | 3 or 4     | ✗ (4 = with token chng) |
/// | CANCEL-ASK   | 1          | ✗                       |
/// | CANCEL-BID   | 1          | ✗                       |
///
/// FULFILL-BID with token change back to taker produces a 4th non-empty output,
/// which is the only signal that distinguishes it from FULFILL-ASK at the spell level.
/// For 3-output fulfills without token change, we default to FulfillAsk.
fn determine_operation(output_orders: &[DexOrder], outs: Option<&Vec<Value>>) -> DexOperation {
    // CREATE: has order charm in outputs (has maker+side+price fields)
    if !output_orders.is_empty() {
        let order = &output_orders[0];
        return match order.side {
            OrderSide::Ask => DexOperation::CreateAskOrder,
            OrderSide::Bid => DexOperation::CreateBidOrder,
        };
    }

    let outs_count = outs.map(|o| o.len()).unwrap_or(0);

    // FULFILL: 3+ outputs (taker addr + maker addr + fee addr)
    // FULFILL-BID with token change has outs[3] = non-empty token charm to taker
    if outs_count >= 3 {
        if outs_count >= 4 {
            if let Some(out3) = outs.and_then(|o| o.get(3)) {
                if out3.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
                    return DexOperation::FulfillBid;
                }
            }
        }
        // 3 outputs (or 4+ without a non-empty outs[3]) → FULFILL-ASK
        // Note: FULFILL-BID with exact token amount also produces 3 outs and
        // is indistinguishable here — treated as FulfillAsk (rare edge case).
        return DexOperation::FulfillAsk;
    }

    // CANCEL: 1-2 outputs (maker gets assets back)
    DexOperation::CancelOrder
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_price() {
        let price_json = json!([1, 500000]);
        let price = parse_price(Some(&price_json));
        assert_eq!(price, Some((1, 500000)));
    }

    #[test]
    fn test_parse_exec_type_all_or_none() {
        let exec = json!("all_or_none");
        assert!(matches!(parse_exec_type(Some(&exec)), ExecType::AllOrNone));
    }

    #[test]
    fn test_parse_exec_type_partial() {
        let exec = json!({"partial": {"from": null}});
        assert!(matches!(
            parse_exec_type(Some(&exec)),
            ExecType::Partial { from: None }
        ));

        let exec_with_from = json!({"partial": {"from": "abc123:0"}});
        if let ExecType::Partial { from } = parse_exec_type(Some(&exec_with_from)) {
            assert_eq!(from, Some("abc123:0".to_string()));
        } else {
            panic!("Expected Partial exec type");
        }
    }
}
