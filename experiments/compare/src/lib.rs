//! Host-side checks. Not fused_fee, and not a deployable program.

use solana_pubkey::Pubkey;

const FUSED_FEE: &str = "AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9";
const REHEARSAL: &str = "3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD";
const TOY_SHARED: &str = "6URwbPipuA4MJLG7LCRRZuWnms3JZ9cRG3z9indXWz8G";
const TOY_PER: &str = "6YMEjhBqVTMaSRWcmVkLrnHZ22FWEDJEpTeonAg8GKSy";

fn pda(program: &str, seeds: &[&[u8]]) -> Pubkey {
    let id: Pubkey = program.parse().unwrap();
    Pubkey::find_program_address(seeds, &id).0
}

#[test]
fn toy_seeds_do_not_match_rehearsal_accounts() {
    let pool = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let live_global = pda(FUSED_FEE, &[b"global"]);
    let rehearsal_global = pda(REHEARSAL, &[b"global"]);
    let toy_b = pda(TOY_SHARED, &[b"xb", b"global"]);
    let toy_a = pda(TOY_PER, &[b"xa", b"global"]);
    assert_ne!(live_global, rehearsal_global, "program id is part of the PDA");
    assert_ne!(toy_b, rehearsal_global);
    assert_ne!(toy_a, rehearsal_global);
    assert_ne!(toy_a, toy_b);

    let live_pool = pda(FUSED_FEE, &[b"pool", pool.as_ref()]);
    let rehearsal_pool = pda(REHEARSAL, &[b"pool", pool.as_ref()]);
    let toy_pool = pda(TOY_SHARED, &[b"xb", b"pool", pool.as_ref()]);
    assert_ne!(live_pool, rehearsal_pool);
    assert_ne!(toy_pool, rehearsal_pool);

    let live_q = pda(FUSED_FEE, &[b"qstat", mint.as_ref()]);
    let rehearsal_q = pda(REHEARSAL, &[b"qstat", mint.as_ref()]);
    assert_ne!(live_q, rehearsal_q);
    println!("rehearsal_global={rehearsal_global}");
    println!("toy_shared_global={toy_b}");
    println!("toy_per_global={toy_a}");
}
