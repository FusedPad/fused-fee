#!/usr/bin/env python3
"""Rebuild named headers from the Anchor Accounts structs and attach handler remaining lists."""
import json, re, pathlib, hashlib

ROOT = pathlib.Path("/workspace/experiments")
SHARED = (ROOT / "integrated/shared_fee/src/lib.rs").read_text()
PER = (ROOT / "integrated/per_pool_fee/src/lib.rs").read_text()

def structs(text):
    out = {}
    for m in re.finditer(r"#\[derive\(Accounts\)\]\s*(?:#\[[^\]]*\]\s*)*pub struct (\w+)", text):
        name = m.group(1)
        i = text.find("{", m.end())
        depth = 0
        j = i
        while j < len(text):
            if text[j] == "{":
                depth += 1
            elif text[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        body = text[i + 1 : j]
        fields = []
        pending = []
        for line in body.splitlines():
            if re.search(r"\bpub \w+\s*:", line) and not line.strip().startswith("//"):
                mm = re.search(r"pub (\w+)\s*:", line)
                blob = "\n".join(pending + [line])
                typ = line.split(":", 1)[1]
                signer = "Signer" in typ
                writable = bool(re.search(r"#\[account\([^\)]*\bmut\b", blob)) or bool(re.search(r"seeds = \[[^\]]*\][^\]]*bump[^\]]*\]", blob) and "mut" in blob)
                # mut anywhere in the attribute stack
                writable = "mut" in blob and "#[account" in blob
                fields.append({"name": mm.group(1), "signer": signer, "writable": writable, "source": "anchor named field"})
                pending = []
            else:
                pending.append(line)
        out[name] = fields
    return out

SS, PS = structs(SHARED), structs(PER)
assert SS["CrankClaimDamm"] and len(SS["CrankClaimDamm"]) == 18
assert len(PS["CrankPayV3"]) == 9 and len(SS["CrankPayV3"]) == 10
assert "CrankAutoLp" in SS and SS["CrankAutoLp"] == []

def errors(text):
    m = re.search(r"#\[error_code\]\s*pub enum FusedError \{", text)
    i = text.find("{", m.end() - 1)
    depth = 0
    j = i
    while j < len(text):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                break
        j += 1
    body = text[i + 1 : j]
    names = re.findall(r"\n\s*([A-Z][A-Za-z0-9]+),", body)
    msgs = re.findall(r'#\[msg\("([^"]*)"\)\]', body)
    assert len(names) == len(msgs), (len(names), len(msgs))
    return [{"code": 6000 + n, "name": name, "message": msg} for n, (name, msg) in enumerate(zip(names, msgs))]

SE, PE = errors(SHARED), errors(PER)
assert SE[0]["name"] == "BadMint" and SE[0]["code"] == 6000
assert next(e for e in PE if e["name"] == "MigrationFeeShort")["code"] == 6053
assert next(e for e in SE if e["name"] == "AlreadyImported")["code"] == 6053

POLICY = {
    "check_quote_mint_return": "7 bytes: tier u8, decimals u8, is_token_2022 u8, flags u32 le",
    "flags": {
        "1": "F_TOKEN_2022",
        "2": "F_FREEZE_AUTHORITY",
        "4": "F_PERMANENT_DELEGATE",
        "8": "F_PAUSABLE",
        "16": "F_HOOK_AUTHORITY",
        "32": "F_TRANSFER_FEE",
        "64": "F_DEFAULT_ACCOUNT_STATE",
        "128": "F_SCALED_OR_INTEREST",
        "256": "F_CONFIDENTIAL",
        "512": "F_CLOSE_AUTHORITY",
    },
    "tier_1_if_any_of": "freeze authority, permanent delegate, pausable, hook authority, transfer fee, default account state. Tier 1 does not by itself reject the mint.",
    "rejects": [
        {"error": "WrongTokenProgram", "code": 6035, "when": "mint owner is not Tokenkeg or Token-2022"},
        {"error": "BadMint", "code": 6000, "when": "mint data shorter than 82, initialized flag not 1, or a Token-2022 TLV entry is truncated"},
        {"error": "UnsupportedMint", "code": 6028, "when": "decimals above 18, the Token-2022 wSOL mint, NonTransferable, or ConfidentialMintBurn"},
        {"error": "HookActive", "code": 6029, "when": "TransferHook program id is not all zeros"},
        {"error": "MintPaused", "code": 6030, "when": "Pausable extension paused byte is not 0"},
        {"error": "DefaultFrozen", "code": 6031, "when": "DefaultAccountState is Frozen (2)"},
        {"error": "TransferFeeTooHigh", "code": 6032, "when": "current or scheduled transfer fee is above 100 bps"},
        {"error": "UnknownExtension", "code": 6033, "when": "any other Token-2022 extension type"},
    ],
    "caller": "The crank pair check builds this instruction and simulates it. It is not a website send. A successful simulation is the gate; a rejecting error suspends the pair.",
}

def R(index, name, signer, writable, **extra):
    d = {"index": index, "name": name, "signer": signer, "writable": writable, "source": "handler remaining_accounts"}
    d.update(extra)
    return d

DAMM_REMAINING_SHARED = [
    R(0, "payer", True, True, note="handler requires signer and writable"),
    R(1, "system_program", False, False),
    R(2, "dbc_pool", False, False, note="must equal pool_record.pool"),
    R(3, "dbc_config", False, False, note="must equal the DBC pool config and pool_record.config"),
    R(4, "creator_owed", False, True, pda=["cown", "pool"], note="written when the 8 percent creator share is nonzero"),
    R(5, "quote_reserve", False, True, pda=["qres", "quote_mint"], note="shared only. The current 18+5 caller does not pass this account."),
]
DAMM_REMAINING_PER = [
    R(0, "payer", True, True),
    R(1, "system_program", False, False),
    R(2, "dbc_pool", False, False),
    R(3, "dbc_config", False, False),
    R(4, "creator_owed", False, True, pda=["cown", "pool"]),
    R(5, "pool_vault", False, True, note="per-pool only. ATA of the pool record. The current 18+5 caller does not pass this account."),
]

AUTO_HEADER = [
    R(0, "global", False, False, pda=["global"]),
    R(1, "auto_lp", False, False, pda=["autolp"]),
    R(2, "payer", True, False, note="admin or crank. Handler checks is_signer, not writability."),
    R(3, "damm_pool", False, True),
    R(4, "position", False, True),
    R(5, "position_nft_account", False, False),
    R(6, "token_a_account", False, True, note="global ATA of token_a_mint"),
    R(7, "token_b_account", False, True, note="global ATA of token_b_mint"),
    R(8, "token_a_vault", False, True),
    R(9, "token_b_vault", False, True),
    R(10, "token_a_mint", False, False),
    R(11, "token_b_mint", False, False),
    R(12, "token_a_program", False, False),
    R(13, "token_b_program", False, False),
    R(14, "event_authority", False, False),
    R(15, "damm_program", False, False, address="cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"),
    R(16, "quote_stat_a", False, False, pda=["qstat", "token_a_mint"]),
    R(17, "quote_stat_b", False, False, pda=["qstat", "token_b_mint"]),
]
AUTO_SHARED_TAIL = AUTO_HEADER + [
    R(18, "quote_reserve_a", False, True, pda=["qres", "token_a_mint"]),
    R(19, "quote_reserve_b", False, True, pda=["qres", "token_b_mint"]),
    R("20+2*i", "pool_record", False, True, pda=["pool", "pool"], note="then creator owed. 1 to 4 pools total across both mints."),
    R("21+2*i", "creator_owed", False, False, pda=["cown", "pool"], note="empty means owed 0. Writable is not required by the handler."),
]
AUTO_PER_TAIL = AUTO_HEADER + [
    R("18+3*i", "pool_record", False, True, pda=["pool", "pool"]),
    R("19+3*i", "pool_vault", False, True, note="ATA of that pool record"),
    R("20+3*i", "creator_owed", False, False, pda=["cown", "pool"], note="empty means owed 0"),
]

LP_OPEN_SHARED = [
    R("8+6*i", "pool_record", False, True),
    R("9+6*i", "source_vault", False, True, note="global ATA of the pool quote"),
    R("10+6*i", "mint", False, False),
    R("11+6*i", "escrow", False, True),
    R("12+6*i", "quote_reserve", False, True, pda=["qres", "quote_mint"]),
    R("13+6*i", "configured_vault", False, True),
]
LP_OPEN_PER = [
    R("8+5*i", "pool_record", False, True),
    R("9+5*i", "pool_vault", False, True),
    R("10+5*i", "mint", False, False),
    R("11+5*i", "escrow", False, True),
    R("12+5*i", "configured_vault", False, True),
]

CREATOR_PER = [
    R(0, "global", False, False, pda=["global"]),
    R(1, "pool_record", False, False, pda=["pool", "pool"]),
    R(2, "creator_owed", False, True, pda=["cown", "pool"]),
    R(3, "dbc_pool", False, False),
    R(4, "pool_vault", False, True, note="owner must be the pool record"),
    R(5, "quote_mint", False, False),
    R(6, "creator_quote", False, True),
    R(7, "quote_token_program", False, False),
]
CREATOR_SHARED = CREATOR_PER[:4] + [
    R(4, "global_vault", False, True, note="owner is the global PDA"),
    R(5, "quote_mint", False, False),
    R(6, "creator_quote", False, True),
    R(7, "quote_token_program", False, False),
    R(8, "quote_reserve", False, True, pda=["qres", "quote_mint"]),
]

STRUCT = {
    "initialize": "Initialize",
    "register_config": "RegisterConfig",
    "update_config": "UpdateConfig",
    "register_pool": "RegisterPool",
    "set_burn_mint": "Admin",
    "set_paused": "Admin",
    "release_burn_pot": "ReleaseBurnPot",
    "migrate_global": "MigrateGlobal",
    "mark_graduated": "MarkGraduated",
    "crank_claim": "CrankClaim",
    "crank_claim_damm": "CrankClaimDamm",
    "crank_pay_page": "CrankPayRetired",
    "crank_pay_page_v2": "CrankPayV2",
    "crank_pay_page_v3": "CrankPayV3",
    "release_burn_pot_v2": "ReleaseBurnPotV2",
    "check_quote_mint": "CheckQuoteMint",
    "close_old_checkpoints": "CloseOldCheckpoints",
    "set_pool_config": "SetPoolConfig",
    "admin_set_graduated": "AdminPool",
    "crank_burn_direct": "CrankBurnDirect",
    "init_auto_lp": "InitAutoLp",
    "set_auto_lp": "SetAutoLp",
    "set_auto_lp_share": "SetAutoLpShare",
    "sync_quote_stat": "SyncQuoteStat",
    "crank_auto_lp": "CrankAutoLp",
    "init_quote_reserve": "InitQuoteReserve",
    "import_quote_reserve": "ImportQuoteReserve",
    "import_existing_pool": "ImportQuoteReserve",
    "seal_quote_reserve": "SealQuoteReserve",
    "open_lp_reservation": "LpBatchIx",
    "consume_lp_reservation": "LpBatchIx",
    "refund_lp_reservation": "LpBatchIx",
    "close_lp_escrow": "LpBatchIx",
    "close_lp_batch": "LpBatchIx",
    "migrate_pool_custody": "LpBatchIx",
    "reverse_pool_custody": "LpBatchIx",
    "claim_creator_share": "ClaimCreatorShare",
}

path = ROOT / "abi/INSTRUCTION-CONTRACT.json"
doc = json.loads(path.read_text())
by_id = {i["id"]: i for i in doc["instructions"]}

def named_from(design_structs, name):
    fields = design_structs[STRUCT[name]]
    accounts = []
    for n, f in enumerate(fields):
        accounts.append({"index": n, "name": f["name"], "signer": f["signer"], "writable": f["writable"], "source": "anchor named field"})
    return accounts

for item in doc["instructions"]:
    design = item["id"].split(".", 1)[0]
    name = item["name"]
    src = SS if design == "shared" else PS
    item["anchor_struct"] = STRUCT[name]
    item["accounts"] = named_from(src, name)
    item["named_len"] = len(item["accounts"])
    item["schema"] = "custom raw ABI derived from the experimental source. Not a generated Anchor IDL. cargo-build-sbf did not emit an IDL for this crate."

# Attach the handler tails that the named context does not carry.
def set_rem(iid, rem, note):
    item = by_id[iid]
    item["remaining"] = rem
    item["handler_note"] = note

set_rem("shared.crank_claim", [R(0, "quote_reserve", False, True, pda=["qres", "quote_mint"])], "16 named accounts are the DBC claim. The 17th account is the sealed reserve.")
set_rem("per_pool.crank_claim", [R(0, "pool_vault", False, True)], "16 named accounts are the DBC claim. The 17th account is this pool's vault. The claim lands in the global ATA, then the net receipt is swept.")
set_rem("shared.crank_claim_damm", DAMM_REMAINING_SHARED, "Named header is 18 accounts, the same 18 the current caller sends. The five creator extras follow. A sixth remaining account, the quote reserve, is required by this source and is not in that 18+5 call.")
set_rem("per_pool.crank_claim_damm", DAMM_REMAINING_PER, "Named header is 18 accounts. The five creator extras follow. A sixth remaining account, the pool vault, is required by this source and is not in that 18+5 call.")
set_rem("shared.crank_pay_page_v2", [R("2*i", "holder_base_account", False, False), R("2*i+1", "destination", False, True, note="writable. Wallet for wSOL, otherwise the holder quote account.")], "quote_reserve is named account index 10, not a remaining account. Empty remaining is the cursor-only page.")
set_rem("per_pool.crank_pay_page_v2", [R("2*i", "holder_base_account", False, False), R("2*i+1", "destination", False, True)], "No quote reserve. The quote vault owner must be the pool record. The current shared-vault caller cannot pay this layout.")
set_rem("shared.crank_pay_page_v3", [R("2*i", "holder_base_account", False, False), R("2*i+1", "holder_quote_account", False, True)], "Ten named accounts. Index 5 is quote_token_program, not token_program. Index 6 is base_mint. Index 7 is system_program. Index 9 is writable quote_reserve. The current caller sends the first nine and does not send the reserve.")
set_rem("per_pool.crank_pay_page_v3", [R("2*i", "holder_base_account", False, False), R("2*i+1", "holder_quote_account", False, True)], "Nine named accounts. This matches the current v3 caller header: payer, global, pool, quote vault, quote mint, quote token program, base mint, system program, pay cursor. The vault owner must be the pool record.")
set_rem("shared.release_burn_pot", [R(0, "quote_reserve", False, True, pda=["qres", "quote_mint"])], "Named vault is the global ATA. The reserve is remaining, not named.")
set_rem("per_pool.release_burn_pot", [], "No remaining account. The vault owner must be the pool record and the pool PDA signs.")
set_rem("shared.release_burn_pot_v2", [R(0, "quote_reserve", False, True, pda=["qres", "quote_mint"])], "Named field token_program must equal the quote mint's owner program.")
set_rem("per_pool.release_burn_pot_v2", [], "Vault owner is the pool record. No reserve.")
set_rem("shared.crank_burn_direct", [R(0, "quote_reserve", False, True, pda=["qres", "quote_mint"])], "Vault is the global ATA.")
set_rem("per_pool.crank_burn_direct", [], "Vault owner is the pool record. This is not the DBC buy-and-burn route.")
set_rem("shared.crank_auto_lp", AUTO_SHARED_TAIL, "Anchor context has no named fields. Every account is remaining, in this order. At most 4 pools. The CPI target in these tests is mock_meteora, not Meteora.")
set_rem("per_pool.crank_auto_lp", AUTO_PER_TAIL, "Anchor context has no named fields. Header matches shared through the two QuoteStats. The tail is pool, pool vault, creator owed, at most 4 pools. No quote reserve. The CPI target in these tests is mock_meteora.")
set_rem("shared.open_lp_reservation", LP_OPEN_SHARED, "Header is the 8 named-by-handler accounts in the accounts array, including lpnonce at index 7. Groups start at index 8. 1 to 4 groups.")
set_rem("per_pool.open_lp_reservation", LP_OPEN_PER, "Same 8-account header. Groups of 5 start at index 8. 1 to 4 groups.")
set_rem("shared.consume_lp_reservation", [R("6+2*i", "escrow", False, True), R("7+2*i", "destination", False, True, note="must equal the vault stored at open")], "Header accounts 0..5 are in the accounts array.")
set_rem("per_pool.consume_lp_reservation", [R("6+2*i", "escrow", False, True), R("7+2*i", "destination", False, True)], "Same pair layout as shared.")
set_rem("shared.refund_lp_reservation", [
    R("4+4*i", "escrow", False, True), R("5+4*i", "source_vault", False, True),
    R("6+4*i", "pool_record", False, True), R("7+4*i", "quote_reserve", False, True),
], "Any signer. Destination must be the source stored at open.")
set_rem("per_pool.refund_lp_reservation", [
    R("4+3*i", "escrow", False, True), R("5+3*i", "source_vault", False, True), R("6+3*i", "pool_record", False, True),
], "Any signer. No reserve.")
set_rem("shared.close_lp_batch", [R("5+i", "escrow", False, True, note="skipped if already closed; rejected if the amount is nonzero")], "lpnonce is handler account 4.")
set_rem("per_pool.close_lp_batch", [R("5+i", "escrow", False, True)], "Same as shared.")
set_rem("shared.claim_creator_share", CREATOR_SHARED, "No named Anchor fields. Global PDA signs. Destination is the DBC pool creator's quote account.")
set_rem("per_pool.claim_creator_share", CREATOR_PER, "No named Anchor fields. Pool PDA signs. Vault owner is the pool record.")
set_rem("shared.crank_claim", by_id["shared.crank_claim"]["remaining"], by_id["shared.crank_claim"]["handler_note"])
set_rem("shared.register_pool", [
    R(0, "quote_mint", False, False, note="instruction index 7. Not the LP watermark."),
    R(1, "quote_stat", False, True, pda=["qstat", "quote_mint"]),
    R(2, "quote_reserve", False, True, pda=["qres", "quote_mint"]),
], "Seven named accounts, then these three. The live client stops after the quote mint.")
set_rem("per_pool.register_pool", [
    R(0, "quote_mint", False, False, note="instruction index 7. Not the LP watermark."),
    R(1, "quote_stat", False, True, pda=["qstat", "quote_mint"]),
], "No quote reserve on this design.")
set_rem("shared.import_quote_reserve", [
    R(0, "qimp", False, True, pda=["qimp", "pool"]),
    R(1, "system_program", False, False),
], "owed is a named writable account, the canonical cown.")
set_rem("shared.import_existing_pool", [
    R(0, "qimp", False, True, pda=["qimp", "pool"]),
    R(1, "system_program", False, False),
], "Same cown rule. Also increments pool_count.")
set_rem("shared.close_old_checkpoints", [R("one per entry, in the same order as entries", "checkpoint", False, True, pda=["ckpt", "pool", "round_le", "owner"])], "The program does not impose a vector cap. The maintenance command chunks at 12. That cap is the command's, not this program's.")
set_rem("per_pool.close_old_checkpoints", [R("one per entry, in the same order as entries", "checkpoint", False, True, pda=["ckpt", "pool", "round_le", "owner"])], "Same as shared. No quote reserve.")

by_id["shared.check_quote_mint"]["handler_note"] = "Used by the crank as a simulate-only pair gate. Not a website send. Return layout and rejecting errors are the policy."
by_id["per_pool.check_quote_mint"]["handler_note"] = by_id["shared.check_quote_mint"]["handler_note"]
by_id["shared.close_old_checkpoints"]["handler_note"] = "Sent by the separate checkpoint maintenance command, not by the website and not by LP batch close."
by_id["per_pool.close_old_checkpoints"]["handler_note"] = by_id["shared.close_old_checkpoints"]["handler_note"]
for iid in ("shared.mark_graduated", "per_pool.mark_graduated"):
    by_id[iid]["handler_note"] = "Three named accounts, no arguments, same on both designs. The crank passes this shape."

doc["schema"] = "custom raw ABI read from the experimental Rust source at commit 380a54d. Not a generated Anchor IDL. The sbf build does not emit one."
doc["elfs_unchanged"] = {
    "shared_3evo_sha256": "d87ce272c943bb7b115cc7c2555f1977e38244d2beac576d727d4a7347f834b6",
    "shared_3evo_bytes": 621672,
    "per_pool_3evo_sha256": "225c7768650666540c5f66d82ea2c4ac2be7567f9189ff998648abc1cc63fba9",
    "per_pool_3evo_bytes": 607824,
    "shared_experiment_sha256": "cb0b1d4952e8123a29a2e8cd80d243faab4c8be54c90cb07b215ae0de4ada92c",
    "shared_experiment_bytes": 620392,
    "per_pool_experiment_sha256": "321acc6eedf0d8a510c968eac64a6e853e75f3a6138a3625d18eeb7f5fff13bf",
    "per_pool_experiment_bytes": 606544,
}
doc["deliberate_incompatibility"] = [
    "Shared crank_pay_page_v3 has 10 named accounts. The current caller sends 9. The tenth is the writable quote reserve. Per-pool has those same 9 and no reserve.",
    "crank_claim_damm has 18 named accounts on both designs, then 5 creator extras, then one design account: quote reserve on shared, pool vault on per-pool. A caller that stops at 18+5 is short one account.",
    "crank_auto_lp has no named accounts. Shared and per-pool tails differ after the two QuoteStats.",
    "Per-pool pay, burn, and release require the quote vault owner to be the pool record. Shared requires the global ATA.",
    "check_quote_mint is a simulate-only crank gate. close_old_checkpoints is a separate maintenance send. Neither is a website instruction, and neither is unused.",
]
doc["errors"] = {"shared": SE, "per_pool": PE, "numbering": "Anchor starts at 6000. The two programs share codes through 6049. From 6050 the variants differ."}
doc["mint_policy"] = POLICY
doc["check_quote_mint_and_close_old_checkpoints"] = "The crank simulates check_quote_mint before a pair is allowed. close_old_checkpoints is sent by its own maintenance command. The earlier handoff sentence that called them unused is wrong."

# Validate every non-empty struct against the exported named header.
problems = []
for item in doc["instructions"]:
    design, name = item["id"].split(".", 1)
    src = SS if design == "shared" else PS
    fields = src[STRUCT[name]]
    got = [(a["name"], a["signer"], a["writable"]) for a in item["accounts"]]
    exp = [(f["name"], f["signer"], f["writable"]) for f in fields]
    if got != exp:
        problems.append((item["id"], exp, got))
    if name == "crank_auto_lp" and not item.get("remaining"):
        problems.append((item["id"], "empty remaining", None))
    if name in ("crank_claim_damm", "crank_pay_page_v3") and "remaining" not in item:
        problems.append((item["id"], "missing remaining", None))
assert not problems, problems
assert by_id["shared.crank_claim_damm"]["named_len"] == 18
assert by_id["per_pool.crank_pay_page_v3"]["named_len"] == 9
assert by_id["shared.crank_pay_page_v3"]["named_len"] == 10
assert by_id["shared.crank_pay_page_v3"]["accounts"][5]["name"] == "quote_token_program"
assert by_id["per_pool.crank_auto_lp"]["named_len"] == 0
assert len(by_id["per_pool.crank_auto_lp"]["remaining"]) >= 18

path.write_text(json.dumps(doc, indent=2) + "\n")
print("wrote", path, "instructions", len(doc["instructions"]), "shared errors", len(SE), "per errors", len(PE))
