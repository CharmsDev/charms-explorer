//! DEX detection logic for Charms Cast orders
//!
//! This module analyzes normalized spells to detect DEX operations
//! such as order creation, fulfillment, cancellation, and partial fills.

use serde_json::Value;

use super::types::{
    DexDetectionResult, DexOperation, DexOrder, ExecType, OrderSide, is_dex_app_id,
};

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

    // Get transaction inputs and outputs
    let tx = native_data.get("tx")?;
    let ins = tx.get("ins").and_then(|v| v.as_array());
    let outs = tx.get("outs").and_then(|v| v.as_array());

    // Analyze inputs and outputs to determine operation type
    let input_orders = extract_orders_from_inputs(ins, &dex_app_id);
    let output_orders = extract_orders_from_outputs(outs, &dex_app_id);

    let operation = determine_operation(&input_orders, &output_orders);

    // Build tags - only product tags, not operation types
    // Operation details go in dex_orders table
    let tags = vec!["charms-cast".to_string()];

    // Extract order details from output if creating/modifying
    let order = output_orders.first().cloned();

    // Build input order IDs
    let input_order_ids: Vec<String> = input_orders
        .iter()
        .filter_map(|o| o.scrolls_address.clone())
        .collect();

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

/// Extract order data from transaction inputs
fn extract_orders_from_inputs(ins: Option<&Vec<Value>>, dex_app_id: &str) -> Vec<DexOrder> {
    let mut orders = Vec::new();

    if let Some(inputs) = ins {
        for (idx, input) in inputs.iter().enumerate() {
            if let Some(order) = extract_order_from_charms(input, dex_app_id, idx) {
                orders.push(order);
            }
        }
    }

    orders
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

/// Extract order from input charms
fn extract_order_from_charms(input: &Value, dex_app_id: &str, _idx: usize) -> Option<DexOrder> {
    // Similar to output extraction
    if let Some(obj) = input.as_object() {
        for (_key, charm_data) in obj {
            if let Some(order) = parse_order_data(charm_data, dex_app_id) {
                return Some(order);
            }
        }
    }

    if let Some(charms) = input.get("charms") {
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

/// Determine operation type based on input/output orders
fn determine_operation(input_orders: &[DexOrder], output_orders: &[DexOrder]) -> DexOperation {
    let has_input_order = !input_orders.is_empty();
    let has_output_order = !output_orders.is_empty();

    match (has_input_order, has_output_order) {
        // No input order, has output order -> Creating new order
        (false, true) => {
            let order = &output_orders[0];
            match order.side {
                OrderSide::Ask => DexOperation::CreateAskOrder,
                OrderSide::Bid => DexOperation::CreateBidOrder,
            }
        }
        // Has input order, no output order -> Full fill or cancel
        (true, false) => {
            // Could be cancel or full fill - need more context
            // For now, assume fulfill
            let order = &input_orders[0];
            match order.side {
                OrderSide::Ask => DexOperation::FulfillAsk,
                OrderSide::Bid => DexOperation::FulfillBid,
            }
        }
        // Has both input and output orders -> Partial fill or cancel+replace
        (true, true) => {
            let input_order = &input_orders[0];
            let output_order = &output_orders[0];

            // Check if it's a partial fill (same side, reduced quantity)
            if input_order.side == output_order.side {
                if let ExecType::Partial { from: Some(_) } = &output_order.exec_type {
                    return DexOperation::PartialFill;
                }
                // Could be cancel+replace
                DexOperation::CancelOrder
            } else {
                // Different sides - likely a match/fulfill
                match input_order.side {
                    OrderSide::Ask => DexOperation::FulfillAsk,
                    OrderSide::Bid => DexOperation::FulfillBid,
                }
            }
        }
        // No orders at all - shouldn't happen for DEX tx
        (false, false) => DexOperation::CancelOrder,
    }
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
