use ::std::collections::HashMap;
use tracing_subscriber::EnvFilter;
use tycho_client::{feed::component_tracker::ComponentFilter, stream::TychoStreamBuilder};
use tycho_common::dto::Chain;

type ComponentId = String;
type TokenAddress = String;

#[derive(Default)]
struct Store {
    components: HashMap<ComponentId, tycho_common::dto::ProtocolComponent>,
    balances: HashMap<ComponentId, HashMap<TokenAddress, tycho_common::dto::ComponentBalance>>,
    tvl: HashMap<ComponentId, f64>,
}

fn bytes_debut_to_addr(s: &str) -> String {
    s.trim()
        .strip_prefix("Bytes(")
        .and_then(|x| x.strip_suffix(")"))
        .unwrap_or(s)
        .to_lowercase()
}

fn bytes_debug_to_u128(s: &str) -> u128 {
    let hex = s
        .trim()
        .strip_prefix("Bytes(0x")
        .and_then(|x| x.strip_suffix(")"))
        .unwrap_or("0");
    if hex.is_empty() {
        return 0;
    }
    u128::from_str_radix(hex, 16).unwrap_or(0)
}

fn uniswap_v2_amount_out(
    amount_in: u128,
    reserve_in: u128,
    reserve_out: u128,
    fee_bps: u128,
) -> u128 {
    let fee_denom: u128 = 10_000;
    let amount_in_with_fee = amount_in * (fee_denom - fee_bps);
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = reserve_in * fee_denom + amount_in_with_fee;
    numerator / denominator
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let tycho_url =
        std::env::var("TYCHO_URL").unwrap_or_else(|_| "tycho-beta.propellerheads.xyz".to_string());
    let auth = std::env::var("TYCHO_AUTH_TOKEN").ok(); // optional

    // Track only pools in a TVL band (ETH units)
    let filter = ComponentFilter::Ids(vec![
        "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".to_string()
    ]);

    let (_, mut receiver) = TychoStreamBuilder::new(&tycho_url, Chain::Ethereum)
        .auth_key(auth)
        .exchange("uniswap_v2", filter.clone())
        .build()
        .await
        .expect("Failed to build tycho stream");

    println!("Connected. Waiting for messages...");

    let mut seen_blocks: u64 = 0;
    let mut local_store: Store = Store::default();

    while let Some(msg) = receiver.recv().await {
        // msg is Result<FeedMessage, _>
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                eprintln!("stream error: {e:?}");
                continue;
            }
        };

        seen_blocks += 1;

        // Print high level info per extractor
        println!("--- message #{seen_blocks} ---");
        for (system, ssm) in msg.state_msgs.iter() {
            let block = ssm.header.number;
            let revert = ssm.header.revert;

            let snap_states = ssm.snapshots.states.len();

            let (new_components, deleted_components, component_balances, component_tvl) = ssm
                .deltas
                .as_ref()
                .map(|d| {
                    (
                        d.new_protocol_components.len(),
                        d.deleted_protocol_components.len(),
                        d.component_balances.len(),
                        d.component_tvl.len(),
                    )
                })
                .unwrap_or((0, 0, 0, 0));

            let removed = ssm.removed_components.len();
            println!(
                "system={system}: block={block} revert={revert} snapshots.states={snap_states} \
             new_components={new_components} deleted={deleted_components} \
             balance_updates={component_balances} tvl_updates={component_tvl} removed={removed}"
            );
            // println!("msg: {msg:?}");
            for (component_id, component_state) in ssm.snapshots.states.iter() {
                local_store
                    .components
                    .insert(component_id.clone(), component_state.component.clone());
            }

            // Apply deltas
            if let Some(deltas) = &ssm.deltas {
                for (component_id, component_state) in deltas.new_protocol_components.iter() {
                    local_store
                        .components
                        .insert(component_id.clone(), component_state.clone());
                }
            }

            // Deleted components (Remove)
            if let Some(deltas) = &ssm.deltas {
                for (component_id, _) in deltas.deleted_protocol_components.iter() {
                    local_store.components.remove(component_id);
                }
            }

            // Apply component_balances deltas
            if let Some(deltas) = &ssm.deltas {
                // 1. component_balances updates
                for (component_id, balance_update) in deltas.component_balances.iter() {
                    let entry = local_store
                        .balances
                        .entry(component_id.clone())
                        .or_default();

                    for (token, cb) in balance_update.0.iter() {
                        let token_addr = bytes_debut_to_addr(&format!("{token:?}"));
                        entry.insert(token_addr, cb.clone());
                    }

                    // 2. tvl updates
                    for (component_id, tvl_update) in deltas.component_tvl.iter() {
                        local_store.tvl.insert(component_id.clone(), *tvl_update);
                    }
                }
            }

            let usdc_addr = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
            let weth_addr = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

            let amount_in = 1_000_000u128; // 1 USDC

            // Print when updates arrive
            if let Some(deltas) = &ssm.deltas {
                for cid in deltas.component_balances.keys() {
                    println!("block={block} component={cid}");
                    if let Some(tb) = local_store.balances.get(cid) {
                        for (token, cb) in tb.iter() {
                            println!(
                                "  token={token} balance_float={} modify_tx={:?}",
                                cb.balance_float, cb.modify_tx
                            )
                        }
                        let cb_usdc = tb.get(usdc_addr).unwrap();
                        let cb_weth = tb.get(weth_addr).unwrap();

                        let reserve_usdc = bytes_debug_to_u128(&format!("{:?}", cb_usdc.balance));
                        let reserve_weth = bytes_debug_to_u128(&format!("{:?}", cb_weth.balance));

                        let out = uniswap_v2_amount_out(amount_in, reserve_usdc, reserve_weth, 30);

                        println!("quote: 1 USDC -> {} wei WETH (fee 0.30%)", out);

                        let cid = "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc";
                        let comp = local_store.components.get(cid).expect("component missing");

                        // TEMP: just print comp fields so we can see what Tycho wants
                        println!("component.protocol_system={}", comp.protocol_system);
                        println!("component.id={}", comp.id);
                        println!(
                            "component.static_attributes keys={:?}",
                            comp.static_attributes.keys()
                        );
                    }

                    if let Some(tvl) = local_store.tvl.get(cid) {
                        println!(" tvl={tvl}");
                    }
                }
            }

            let store_size = local_store.components.len();

            println!(
                "block={block} store_size={store_size} is_snapshot={}",
                snap_states > 0
            );

            // Where deltas had updates
            if let Some(deltas) = &ssm.deltas {
                if deltas.component_balances.len() > 0 || deltas.component_tvl.len() > 0 {
                    println!(
                        " deltas: balances={} tvl={}",
                        deltas.component_balances.len(),
                        deltas.component_tvl.len()
                    );
                }
            }
        }
    }

    Ok(())
}
