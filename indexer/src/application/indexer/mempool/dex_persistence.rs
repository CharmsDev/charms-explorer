//! Mempool DEX persistence: saves CREATE orders + activity rows for
//! FULFILL/CANCEL, and corrects 3-output FULFILL classification by looking
//! up the consumed order's side.

use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use crate::domain::services::dex::{self, extract_ins0_order_id, ExecType, OrderSide};
use crate::domain::services::tx_analyzer;
use crate::infrastructure::persistence::entities::dex_orders;
use crate::infrastructure::persistence::error::is_duplicate_key;
use crate::utils::logging;

/// Save a DEX order detected in a mempool transaction.
pub async fn save_dex_order(
    txid: &str,
    analyzed: &tx_analyzer::AnalyzedTx,
    blockchain: &str,
    network: &str,
    db: &DatabaseConnection,
) {
    let dex_result = match &analyzed.dex_result {
        Some(d) => d,
        None => return,
    };
    let order = match &dex_result.order {
        Some(o) => o,
        None => return,
    };

    let order_id = format!("{}:0", txid);
    let now_dt = Utc::now().naive_utc();

    let status = match dex_result.operation {
        dex::DexOperation::CreateAskOrder | dex::DexOperation::CreateBidOrder => "open",
        dex::DexOperation::PartialFill => "partial",
        dex::DexOperation::FulfillAsk | dex::DexOperation::FulfillBid => "filled",
        dex::DexOperation::CancelOrder => "cancelled",
    };

    let side_str = match order.side {
        OrderSide::Ask => "ask",
        OrderSide::Bid => "bid",
    };
    let exec_type_str = match &order.exec_type {
        ExecType::AllOrNone => "all_or_none",
        ExecType::Partial { .. } => "partial",
    };
    let parent_order_id = if let ExecType::Partial { from } = &order.exec_type {
        from.clone()
    } else {
        None
    };

    let order_model = dex_orders::ActiveModel {
        order_id: Set(order_id),
        txid: Set(txid.to_string()),
        vout: Set(0i32),
        block_height: Set(None),
        platform: Set("charms-cast".to_string()),
        maker: Set(order.maker.clone()),
        side: Set(side_str.to_string()),
        exec_type: Set(exec_type_str.to_string()),
        price_num: Set(order.price.0 as i64),
        price_den: Set(order.price.1 as i64),
        amount: Set(order.amount as i64),
        quantity: Set(order.quantity as i64),
        filled_amount: Set(0),
        filled_quantity: Set(0),
        asset_app_id: Set(order.asset_app_id.clone()),
        scrolls_address: Set(order.scrolls_address.clone()),
        status: Set(status.to_string()),
        parent_order_id: Set(parent_order_id),
        created_at: Set(now_dt),
        updated_at: Set(now_dt),
        blockchain: Set(blockchain.to_string()),
        network: Set(network.to_string()),
    };

    match order_model.insert(db).await {
        Ok(_) => {
            logging::log_info(&format!(
                "[{}] 💾 Mempool DEX order saved: {} ({:?})",
                network, txid, dex_result.operation
            ));
            crate::utils::metrics::dex_order_detected(
                network,
                dex_operation_label(&dex_result.operation),
            );
        }
        Err(e) if is_duplicate_key(&e) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to save mempool DEX order {}: {}",
                network, txid, e
            ));
        }
    }
}

/// Label suitable for a Prometheus metric value (snake_case, low cardinality).
fn dex_operation_label(op: &dex::DexOperation) -> &'static str {
    match op {
        dex::DexOperation::CreateAskOrder => "create_ask",
        dex::DexOperation::CreateBidOrder => "create_bid",
        dex::DexOperation::PartialFill => "partial_fill",
        dex::DexOperation::FulfillAsk => "fulfill_ask",
        dex::DexOperation::FulfillBid => "fulfill_bid",
        dex::DexOperation::CancelOrder => "cancel",
    }
}

/// Correct FULFILL-BID misclassified as FulfillAsk (3-output edge case).
///
/// `detect_dex_operation()` returns FulfillAsk for all 3-output fulfills
/// because FULFILL-ASK and FULFILL-BID without token change have identical
/// spell structures. This function looks up the consumed order (ins[0]) in
/// dex_orders and, if its side is "bid", corrects the operation and tag.
pub async fn correct_fulfill_classification(
    txid: &str,
    raw_hex: &str,
    mut analyzed: tx_analyzer::AnalyzedTx,
    network: &str,
    db: &DatabaseConnection,
) -> tx_analyzer::AnalyzedTx {
    let is_fulfill_ask = analyzed
        .dex_result
        .as_ref()
        .is_some_and(|d| d.operation == dex::DexOperation::FulfillAsk);
    if !is_fulfill_ask {
        return analyzed;
    }

    let order_id = match extract_ins0_order_id(raw_hex) {
        Some(id) => id,
        None => return analyzed,
    };

    let order = match dex_orders::Entity::find_by_id(order_id.clone())
        .one(db)
        .await
    {
        Ok(Some(o)) => o,
        _ => return analyzed, // Not found or DB error → keep FulfillAsk
    };

    if order.side == "bid" {
        if let Some(ref mut result) = analyzed.dex_result {
            result.operation = dex::DexOperation::FulfillBid;
        }
        if let Some(ref mut tags) = analyzed.tags {
            *tags = tags.replace("fulfill-ask", "fulfill-bid");
        }
        logging::log_info(&format!(
            "[{}] 🔄 FULFILL-BID (3-out) corrected for tx {} (consumed order {})",
            network, txid, order_id
        ));
    }

    analyzed
}

/// Update the consumed order's status in dex_orders when a FULFILL or CANCEL
/// is detected in the mempool. Also inserts a new activity row for the
/// fulfill/cancel transaction, copying data from the parent order.
///
/// Audit N10: short-circuits when an activity row for this txid already
/// exists (RBF re-detection). Audit N8: the parent's previous status is
/// derived from `filled_amount` on revert, not hard-coded to "open".
pub async fn update_consumed_order_status(
    txid: &str,
    raw_hex: &str,
    analyzed: &tx_analyzer::AnalyzedTx,
    blockchain: &str,
    network: &str,
    db: &DatabaseConnection,
) {
    let new_status = match analyzed.dex_result.as_ref().map(|d| &d.operation) {
        Some(dex::DexOperation::FulfillAsk) | Some(dex::DexOperation::FulfillBid) => "filled",
        Some(dex::DexOperation::CancelOrder) => "cancelled",
        _ => return,
    };

    let order_id = match extract_ins0_order_id(raw_hex) {
        Some(id) => id,
        None => return,
    };

    let order = match dex_orders::Entity::find_by_id(order_id.clone())
        .one(db)
        .await
    {
        Ok(Some(o)) => o,
        Ok(None) => return, // Order not yet indexed (e.g., still in mempool)
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to look up order {} for status update: {}",
                network, order_id, e
            ));
            return;
        }
    };

    let activity_order_id = format!("{}:0", txid);
    match dex_orders::Entity::find_by_id(activity_order_id.clone())
        .one(db)
        .await
    {
        Ok(Some(_)) => return, // already processed
        Ok(None) => {}
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to check existing activity row {}: {}",
                network, activity_order_id, e
            ));
            return;
        }
    }

    let now = chrono::Utc::now().naive_utc();
    let activity_model = dex_orders::ActiveModel {
        order_id: Set(activity_order_id.clone()),
        txid: Set(txid.to_string()),
        vout: Set(0i32),
        block_height: Set(None), // mempool
        platform: Set(order.platform.clone()),
        maker: Set(order.maker.clone()),
        side: Set(order.side.clone()),
        exec_type: Set(order.exec_type.clone()),
        price_num: Set(order.price_num),
        price_den: Set(order.price_den),
        amount: Set(order.amount),
        quantity: Set(order.quantity),
        filled_amount: Set(0),
        filled_quantity: Set(0),
        asset_app_id: Set(order.asset_app_id.clone()),
        scrolls_address: Set(order.scrolls_address.clone()),
        status: Set(new_status.to_string()),
        parent_order_id: Set(Some(order.order_id.clone())),
        created_at: Set(now),
        updated_at: Set(now),
        blockchain: Set(blockchain.to_string()),
        network: Set(network.to_string()),
    };

    match activity_model.insert(db).await {
        Ok(_) => {
            logging::log_info(&format!(
                "[{}] 💾 Mempool activity row saved: {} ({}) parent={}",
                network, txid, new_status, order_id
            ));
        }
        Err(e) if is_duplicate_key(&e) => return, // raced, don't touch parent
        Err(e) => {
            logging::log_warning(&format!(
                "[{}] ⚠️ Failed to save activity row for {}: {}",
                network, txid, e
            ));
            return;
        }
    }

    if order.status == "open" || order.status == "partial" {
        let mut active: dex_orders::ActiveModel = order.into();
        active.status = Set(new_status.to_string());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        match active.update(db).await {
            Ok(_) => {
                logging::log_info(&format!(
                    "[{}] 🔄 Order {} → {} (mempool)",
                    network, order_id, new_status
                ));
            }
            Err(e) => {
                logging::log_warning(&format!(
                    "[{}] ⚠️ Failed to update order {} status to {}: {}",
                    network, order_id, new_status, e
                ));
            }
        }
    }
}
