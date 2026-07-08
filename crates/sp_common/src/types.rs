use soroban_sdk::{contracttype, Address, BytesN, Symbol, Vec};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Trader,
    Organizer,
    Supplier,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Unregistered,
    Registered,
    Verified,
    Suspended,
    Closed,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolState {
    Open,
    Filled,
    Accepted,
    Dispatched,
    Delivered,
    Settled,
    Expired,
    Failed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tier {
    pub min_units: u32,
    pub unit_price: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolTerms {
    pub supplier: Address,
    pub offer_hash: BytesN<32>,
    pub unit_label_hash: BytesN<32>,
    pub category: Symbol,
    pub tiers: Vec<Tier>,
    pub moq: u32,
    pub max_units: u32,
    pub max_per_member: u32,
    pub lead_time_secs: u64,
    pub perishable: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    pub id: u64,
    pub organizer: Address,
    pub terms: PoolTerms,
    pub hub_hash: BytesN<32>,
    pub organizer_fee_bp: u32,
    pub platform_fee_bp: u32,
    pub cluster: Option<Symbol>,
    pub fill_deadline: u64,
    pub created_at: u64,
    pub filled_at: u64,
    pub accepted_at: u64,
    pub dispatched_at: u64,
    pub delivered_at: u64,
    pub total_units: u32,
    pub received_units: u32,
    /// Price per unit fixed when the pool reaches Filled.
    pub final_price: i128,
    /// Sum of all member payments (never decreases except on withdrawal).
    pub total_paid: i128,
    /// Tokens the contract still holds for this pool.
    pub escrow_balance: i128,
    pub frozen_amount: i128,
    /// Dispute outcomes already earmarked for members (pre-settlement).
    pub diverted: i128,
    /// Extra refund shared pro-rata among all members (RefundPool outcome).
    pub extra_refund_pool: i128,
    pub advance_paid: i128,
    /// Members with units > 0.
    pub member_count: u32,
    /// Length of the member list, including withdrawn members.
    pub listed_members: u32,
    pub picked_units: u32,
    pub confirm_units: u32,
    pub alloc_cursor: u32,
    pub alloc_cum: u32,
    pub alloc_done: bool,
    pub refund_cursor: u32,
    /// Total owed back to members once the pool is final (refunds + credits).
    pub owed_total: i128,
    pub refunds_paid: i128,
    pub state: PoolState,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Commitment {
    pub member: Address,
    pub units: u32,
    pub paid: i128,
    pub allocated_units: u32,
    /// Total refund amount already paid out to the member.
    pub refund_claimed: i128,
    /// Extra credit from dispute outcomes.
    pub extra_credit: i128,
    pub picked_up: bool,
    pub delivery_confirmed: bool,
    pub committed_at: u64,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisputeReason {
    Short,
    WrongItem,
    Damaged,
    Quality,
    NotDelivered,
    Other,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    ReleaseToSupplier,
    RefundMember(u32),
    Split(u32),
    RefundPool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub id: u64,
    pub pool_id: u64,
    pub opener: Address,
    pub reason: DisputeReason,
    pub claimed_units: u32,
    pub claimed_amount: i128,
    pub deposit: i128,
    pub evidence: Vec<BytesN<32>>,
    pub opened_at: u64,
    pub resolved: bool,
    /// Empty until resolved, then exactly one element (Option<Outcome> is not supported by the SDK).
    pub outcome: Vec<Outcome>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Params {
    pub platform_fee_bp: u32,
    pub max_organizer_fee_bp: u32,
    pub max_fill_window_secs: u64,
    pub accept_window_secs: u64,
    pub delivery_grace_secs: u64,
    pub confirm_window_secs: u64,
    pub perishable_confirm_window_secs: u64,
    pub organizer_confirm_window_secs: u64,
    pub early_release_weight_bp: u32,
    pub dispute_deposit_bp: u32,
    pub dispute_deposit_min: i128,
    pub arbitration_sla_secs: u64,
    pub withdraw_lock_secs: u64,
    pub max_member_share_bp: u32,
    pub max_members_per_pool: u32,
    pub advance_enabled: bool,
    pub supplier_tier_caps: Vec<i128>,
    pub organizer_tier_caps: Vec<i128>,
    pub trader_tier_caps: Vec<i128>,
    pub supplier_advance_bp: Vec<u32>,
}

#[contracttype]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepStats {
    pub pools_settled: u32,
    pub pools_failed: u32,
    pub on_time: u32,
    pub late: u32,
    pub disputes_opened: u32,
    pub disputes_won: u32,
    pub disputes_lost: u32,
    pub short_deliveries: u32,
    pub pickups_confirmed: u32,
    pub first_active_at: u64,
}

/// Hard caps enforced by `config` (changing them needs a contract upgrade).
pub mod caps {
    pub const PLATFORM_FEE_BP: u32 = 500;
    pub const ORGANIZER_FEE_BP: u32 = 200;
    pub const FILL_WINDOW_SECS: u64 = 14 * 24 * 3600;
    pub const ADVANCE_BP: u32 = 4000;
    pub const ARBITRATION_SLA_SECS: u64 = 14 * 24 * 3600;
    pub const DISPUTE_DEPOSIT_BP: u32 = 500;
    pub const MAX_TIERS: u32 = 5;
}
