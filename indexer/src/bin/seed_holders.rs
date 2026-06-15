//! One-shot backfill: seed every monitored address whose `seeded_at IS NULL`
//! via Maestro. Uses the same `seed_one` path as the live worker.
//!
//! Usage:
//!     cargo run --release --bin seed_holders -- [--network <name>] [--limit N] [--rps R] [--dry-run]
//!
//! Defaults: network=mainnet, no limit, ~5 requests/sec, real run.

use std::time::{Duration, Instant};

use charms_indexer::application::indexer::seeder::worker::{seed_one, SeedError};
use charms_indexer::config::AppConfig;
use charms_indexer::infrastructure::maestro::MaestroClient;
use charms_indexer::infrastructure::persistence::{DbPool, Repositories};
use charms_indexer::utils::logging;

struct Args {
    network: String,
    limit: Option<usize>,
    rps: f64,
    dry_run: bool,
}

fn parse_args() -> Args {
    let mut network = "mainnet".to_string();
    let mut limit: Option<usize> = None;
    let mut rps: f64 = 5.0;
    let mut dry_run = false;

    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--network" => {
                network = raw.get(i + 1).cloned().unwrap_or(network);
                i += 2;
            }
            "--limit" => {
                limit = raw.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--rps" => {
                rps = raw.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(rps);
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                std::process::exit(2);
            }
        }
    }
    Args {
        network,
        limit,
        rps,
        dry_run,
    }
}

#[tokio::main]
async fn main() {
    logging::init_logger();
    let args = parse_args();
    let config = AppConfig::from_env();

    if config.indexer.private_maestro_api_key.is_empty() {
        eprintln!("PRIVATE_MAESTRO_API_KEY is not set; aborting.");
        std::process::exit(1);
    }

    let pool = DbPool::new(&config)
        .await
        .expect("connect to database");
    let repos = Repositories::from_pool(&pool);
    let maestro = MaestroClient::new(config.indexer.private_maestro_api_key.clone());

    let targets: Vec<String> = repos
        .monitored_addresses
        .fetch_unseeded(&args.network, args.limit.map(|l| l as u64).unwrap_or(u64::MAX))
        .await
        .expect("fetch unseeded");

    let total = targets.len();
    if total == 0 {
        println!("Nothing to seed for network={}; all monitored addresses already have seeded_at.", args.network);
        return;
    }

    println!(
        "Seeding {} address(es) for network={} (dry_run={}, rps≈{})",
        total, args.network, args.dry_run, args.rps
    );

    let per_request = Duration::from_millis(((1000.0 / args.rps).max(1.0)) as u64);
    let mut ok = 0usize;
    let mut busy = 0usize;
    let mut err = 0usize;
    let started = Instant::now();
    let mut last_step = Instant::now();

    for (idx, address) in targets.iter().enumerate() {
        if args.dry_run {
            println!("[dry-run] would seed: {}", address);
            ok += 1;
        } else {
            match seed_one(&maestro, &repos, address, &args.network).await {
                Ok(out) => {
                    ok += 1;
                    if (idx + 1) % 25 == 0 {
                        println!(
                            "  ✓ [{}/{}] {} → utxos={}, txs={}, tip={}",
                            idx + 1, total, address, out.utxos, out.txs, out.tip_height
                        );
                    }
                }
                Err(SeedError::LockBusy) => busy += 1,
                Err(e) => {
                    err += 1;
                    eprintln!("  ✗ [{}/{}] {} → {}", idx + 1, total, address, e);
                }
            }
        }

        // Rate limit: each request consumes 1/rps seconds minimum.
        let elapsed_since_last = last_step.elapsed();
        if elapsed_since_last < per_request {
            tokio::time::sleep(per_request - elapsed_since_last).await;
        }
        last_step = Instant::now();
    }

    let total_dur = started.elapsed();
    println!(
        "Done in {:.1}s — ok={} busy={} err={} total={}",
        total_dur.as_secs_f64(),
        ok,
        busy,
        err,
        total
    );
    if err > 0 {
        std::process::exit(3);
    }
}
