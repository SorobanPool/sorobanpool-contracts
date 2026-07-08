import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}




export const Errors = {
  300: {message:"NotInitialised"},
  301: {message:"Paused"},
  302: {message:"BadTerms"},
  303: {message:"CategoryNotAllowed"},
  304: {message:"NotVerified"},
  305: {message:"BadDeadline"},
  306: {message:"FeeTooHigh"},
  307: {message:"PoolTooLarge"},
  308: {message:"PoolNotFound"},
  309: {message:"WrongState"},
  310: {message:"DeadlinePassed"},
  311: {message:"DeadlineNotReached"},
  312: {message:"ZeroUnits"},
  313: {message:"NotAllowedMember"},
  314: {message:"ClusterMismatch"},
  315: {message:"OverMemberLimit"},
  316: {message:"OverMemberShare"},
  317: {message:"OverPoolMax"},
  318: {message:"OverTierCap"},
  319: {message:"TooManyMembers"},
  320: {message:"AlreadyCommitted"},
  321: {message:"NotCommitted"},
  322: {message:"WithdrawLocked"},
  323: {message:"WouldLowerTier"},
  324: {message:"BelowMoq"},
  325: {message:"NotOrganizer"},
  326: {message:"NotSupplier"},
  327: {message:"NotAdmin"},
  328: {message:"CannotCancel"},
  329: {message:"AcceptWindowPassed"},
  330: {message:"AcceptWindowOpen"},
  331: {message:"AdvanceDisabled"},
  332: {message:"AdvanceTooHigh"},
  333: {message:"DeliveryDeadlineNotPassed"},
  334: {message:"BadReceivedUnits"},
  335: {message:"ConfirmDeadlineNotPassed"},
  336: {message:"AlreadyConfirmed"},
  337: {message:"AllocationPending"},
  338: {message:"SettleTooEarly"},
  339: {message:"NothingToClaim"},
  340: {message:"NotDisputes"},
  341: {message:"FreezeTooLarge"},
  342: {message:"DisputeWindowClosed"},
  343: {message:"Overflow"},
  344: {message:"NotFinal"},
  345: {message:"SupplierIsParty"}
}


export interface Pool {
  accepted_at: u64;
  advance_paid: i128;
  alloc_cum: u32;
  alloc_cursor: u32;
  alloc_done: boolean;
  cluster: Option<string>;
  confirm_units: u32;
  created_at: u64;
  delivered_at: u64;
  dispatched_at: u64;
  /**
 * Dispute outcomes already earmarked for members (pre-settlement).
 */
diverted: i128;
  /**
 * Tokens the contract still holds for this pool.
 */
escrow_balance: i128;
  /**
 * Extra refund shared pro-rata among all members (RefundPool outcome).
 */
extra_refund_pool: i128;
  fill_deadline: u64;
  filled_at: u64;
  /**
 * Price per unit fixed when the pool reaches Filled.
 */
final_price: i128;
  frozen_amount: i128;
  hub_hash: Buffer;
  id: u64;
  /**
 * Length of the member list, including withdrawn members.
 */
listed_members: u32;
  /**
 * Members with units > 0.
 */
member_count: u32;
  organizer: string;
  organizer_fee_bp: u32;
  /**
 * Total owed back to members once the pool is final (refunds + credits).
 */
owed_total: i128;
  picked_units: u32;
  platform_fee_bp: u32;
  received_units: u32;
  refund_cursor: u32;
  refunds_paid: i128;
  state: PoolState;
  terms: PoolTerms;
  /**
 * Sum of all member payments (never decreases except on withdrawal).
 */
total_paid: i128;
  total_units: u32;
}


export interface Tier {
  min_units: u32;
  unit_price: i128;
}

export type PoolState = {tag: "Open", values: void} | {tag: "Filled", values: void} | {tag: "Accepted", values: void} | {tag: "Dispatched", values: void} | {tag: "Delivered", values: void} | {tag: "Settled", values: void} | {tag: "Expired", values: void} | {tag: "Failed", values: void} | {tag: "Cancelled", values: void};


export interface PoolTerms {
  category: string;
  lead_time_secs: u64;
  max_per_member: u32;
  max_units: u32;
  moq: u32;
  offer_hash: Buffer;
  perishable: boolean;
  supplier: string;
  tiers: Array<Tier>;
  unit_label_hash: Buffer;
}


export interface Commitment {
  allocated_units: u32;
  committed_at: u64;
  delivery_confirmed: boolean;
  /**
 * Extra credit from dispute outcomes.
 */
extra_credit: i128;
  member: string;
  paid: i128;
  picked_up: boolean;
  /**
 * Total refund amount already paid out to the member.
 */
refund_claimed: i128;
  units: u32;
}

export interface Client {
  /**
   * Construct and simulate a pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  pool: ({id}: {id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<Pool>>

  /**
   * Construct and simulate a close transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless once the fill deadline has passed.
   */
  close: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a accept transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * `advance_bp` is 0 for no advance; advances are off unless `advance_enabled`.
   */
  accept: ({supplier, pool_id, advance_bp}: {supplier: string, pool_id: u64, advance_bp: u32}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a commit transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  commit: ({member, pool_id, units}: {member: string, pool_id: u64, units: u32}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a freeze transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  freeze: ({caller, pool_id, amount}: {caller: string, pool_id: u64, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a reject transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  reject: ({supplier, pool_id}: {supplier: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a settle transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless once the confirmation window has closed, or early when members holding
   * `early_release_weight_bp` of units have confirmed pickup. Frozen amounts stay in escrow.
   */
  settle: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a members transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  members: ({pool_id, start, limit}: {pool_id: u64, start: u32, limit: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Array<string>>>

  /**
   * Construct and simulate a dispatch transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  dispatch: ({supplier, pool_id, waybill_hash}: {supplier: string, pool_id: u64, waybill_hash: Option<Buffer>}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a increase transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  increase: ({member, pool_id, extra_units}: {member: string, pool_id: u64, extra_units: u32}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a commitment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  commitment: ({pool_id, member}: {pool_id: u64, member: string}, options?: MethodOptions) => Promise<AssembledTransaction<Option<Commitment>>>

  /**
   * Construct and simulate a sweep_dust transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Sends rounding dust (escrow beyond what is still owed) to the treasury.
   */
  sweep_dust: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a cancel_pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer if nothing is committed yet; admin any time before Dispatched.
   */
  cancel_pool: ({caller, pool_id}: {caller: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a close_early transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  close_early: ({organizer, pool_id}: {organizer: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a create_pool transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  create_pool: ({organizer, terms, hub_hash, organizer_fee_bp, cluster, fill_deadline}: {organizer: string, terms: PoolTerms, hub_hash: Buffer, organizer_fee_bp: u32, cluster: Option<string>, fill_deadline: u64}, options?: MethodOptions) => Promise<AssembledTransaction<u64>>

  /**
   * Construct and simulate a fail_accept transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless after the accept window.
   */
  fail_accept: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a claim_refund transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  claim_refund: ({member, pool_id}: {member: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a current_tier transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  current_tier: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<readonly [u32, i128]>>

  /**
   * Construct and simulate a push_refunds transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless bounded batch push of refunds. Returns members processed.
   */
  push_refunds: ({pool_id, max}: {pool_id: u64, max: u32}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a quote_commit transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  quote_commit: ({pool_id, units}: {pool_id: u64, units: u32}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a apply_outcome transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  apply_outcome: ({caller, pool_id, member, to_member, to_supplier}: {caller: string, pool_id: u64, member: Option<string>, to_member: i128, to_supplier: i128}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a fail_delivery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless after `accepted_at + lead_time + grace`.
   */
  fail_delivery: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a refundable_of transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  refundable_of: ({pool_id, member}: {pool_id: u64, member: string}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a confirm_pickup transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  confirm_pickup: ({member, pool_id}: {member: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a payout_preview transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * (supplier, platform, organizer, refunds) if the pool settled now.
   */
  payout_preview: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<readonly [i128, i128, i128, i128]>>

  /**
   * Construct and simulate a confirm_deadline transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  confirm_deadline: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<u64>>

  /**
   * Construct and simulate a confirm_delivery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  confirm_delivery: ({organizer, pool_id, received_units, evidence}: {organizer: string, pool_id: u64, received_units: u32, evidence: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a allocate_shortfall transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless, paged. Needed only after a shortfall; fills `allocated_units`.
   */
  allocate_shortfall: ({pool_id, max}: {pool_id: u64, max: u32}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a allocated_units_of transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  allocated_units_of: ({pool_id, member}: {pool_id: u64, member: string}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a withdraw_commitment transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Allowed while Open and before the lock, and only if it does not lower the pool's price
   * tier (ADR 0001): that keeps every member's paid amount at or above the final bill.
   */
  withdraw_commitment: ({member, pool_id}: {member: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a member_confirm_delivery transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Collective fallback when the organizer has not confirmed in time: members holding at
   * least 50% of units can confirm that the full order arrived.
   */
  member_confirm_delivery: ({member, pool_id}: {member: string, pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
        /** Constructor/Initialization Args for the contract's `__constructor` method */
        {config}: {config: string},
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy({config}, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAALgAAAAAAAAAOTm90SW5pdGlhbGlzZWQAAAAAASwAAAAAAAAABlBhdXNlZAAAAAABLQAAAAAAAAAIQmFkVGVybXMAAAEuAAAAAAAAABJDYXRlZ29yeU5vdEFsbG93ZWQAAAAAAS8AAAAAAAAAC05vdFZlcmlmaWVkAAAAATAAAAAAAAAAC0JhZERlYWRsaW5lAAAAATEAAAAAAAAACkZlZVRvb0hpZ2gAAAAAATIAAAAAAAAADFBvb2xUb29MYXJnZQAAATMAAAAAAAAADFBvb2xOb3RGb3VuZAAAATQAAAAAAAAACldyb25nU3RhdGUAAAAAATUAAAAAAAAADkRlYWRsaW5lUGFzc2VkAAAAAAE2AAAAAAAAABJEZWFkbGluZU5vdFJlYWNoZWQAAAAAATcAAAAAAAAACVplcm9Vbml0cwAAAAAAATgAAAAAAAAAEE5vdEFsbG93ZWRNZW1iZXIAAAE5AAAAAAAAAA9DbHVzdGVyTWlzbWF0Y2gAAAABOgAAAAAAAAAPT3Zlck1lbWJlckxpbWl0AAAAATsAAAAAAAAAD092ZXJNZW1iZXJTaGFyZQAAAAE8AAAAAAAAAAtPdmVyUG9vbE1heAAAAAE9AAAAAAAAAAtPdmVyVGllckNhcAAAAAE+AAAAAAAAAA5Ub29NYW55TWVtYmVycwAAAAABPwAAAAAAAAAQQWxyZWFkeUNvbW1pdHRlZAAAAUAAAAAAAAAADE5vdENvbW1pdHRlZAAAAUEAAAAAAAAADldpdGhkcmF3TG9ja2VkAAAAAAFCAAAAAAAAAA5Xb3VsZExvd2VyVGllcgAAAAABQwAAAAAAAAAIQmVsb3dNb3EAAAFEAAAAAAAAAAxOb3RPcmdhbml6ZXIAAAFFAAAAAAAAAAtOb3RTdXBwbGllcgAAAAFGAAAAAAAAAAhOb3RBZG1pbgAAAUcAAAAAAAAADENhbm5vdENhbmNlbAAAAUgAAAAAAAAAEkFjY2VwdFdpbmRvd1Bhc3NlZAAAAAABSQAAAAAAAAAQQWNjZXB0V2luZG93T3BlbgAAAUoAAAAAAAAAD0FkdmFuY2VEaXNhYmxlZAAAAAFLAAAAAAAAAA5BZHZhbmNlVG9vSGlnaAAAAAABTAAAAAAAAAAZRGVsaXZlcnlEZWFkbGluZU5vdFBhc3NlZAAAAAAAAU0AAAAAAAAAEEJhZFJlY2VpdmVkVW5pdHMAAAFOAAAAAAAAABhDb25maXJtRGVhZGxpbmVOb3RQYXNzZWQAAAFPAAAAAAAAABBBbHJlYWR5Q29uZmlybWVkAAABUAAAAAAAAAARQWxsb2NhdGlvblBlbmRpbmcAAAAAAAFRAAAAAAAAAA5TZXR0bGVUb29FYXJseQAAAAABUgAAAAAAAAAOTm90aGluZ1RvQ2xhaW0AAAAAAVMAAAAAAAAAC05vdERpc3B1dGVzAAAAAVQAAAAAAAAADkZyZWV6ZVRvb0xhcmdlAAAAAAFVAAAAAAAAABNEaXNwdXRlV2luZG93Q2xvc2VkAAAAAVYAAAAAAAAACE92ZXJmbG93AAABVwAAAAAAAAAITm90RmluYWwAAAFYAAAAAAAAAA9TdXBwbGllcklzUGFydHkAAAABWQ==",
        "AAAAAAAAAAAAAAAEcG9vbAAAAAEAAAAAAAAAAmlkAAAAAAAGAAAAAQAAB9AAAAAEUG9vbA==",
        "AAAAAAAAADFQZXJtaXNzaW9ubGVzcyBvbmNlIHRoZSBmaWxsIGRlYWRsaW5lIGhhcyBwYXNzZWQuAAAAAAAABWNsb3NlAAAAAAAAAQAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAA==",
        "AAAAAAAAAExgYWR2YW5jZV9icGAgaXMgMCBmb3Igbm8gYWR2YW5jZTsgYWR2YW5jZXMgYXJlIG9mZiB1bmxlc3MgYGFkdmFuY2VfZW5hYmxlZGAuAAAABmFjY2VwdAAAAAAAAwAAAAAAAAAIc3VwcGxpZXIAAAATAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAAAAAACmFkdmFuY2VfYnAAAAAAAAQAAAAA",
        "AAAAAAAAAAAAAAAGY29tbWl0AAAAAAADAAAAAAAAAAZtZW1iZXIAAAAAABMAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAAFdW5pdHMAAAAAAAAEAAAAAQAAAAs=",
        "AAAAAAAAAAAAAAAGZnJlZXplAAAAAAADAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAAGYW1vdW50AAAAAAALAAAAAA==",
        "AAAAAAAAAAAAAAAGcmVqZWN0AAAAAAACAAAAAAAAAAhzdXBwbGllcgAAABMAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAA=",
        "AAAAAAAAAK5QZXJtaXNzaW9ubGVzcyBvbmNlIHRoZSBjb25maXJtYXRpb24gd2luZG93IGhhcyBjbG9zZWQsIG9yIGVhcmx5IHdoZW4gbWVtYmVycyBob2xkaW5nCmBlYXJseV9yZWxlYXNlX3dlaWdodF9icGAgb2YgdW5pdHMgaGF2ZSBjb25maXJtZWQgcGlja3VwLiBGcm96ZW4gYW1vdW50cyBzdGF5IGluIGVzY3Jvdy4AAAAAAAZzZXR0bGUAAAAAAAEAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAA=",
        "AAAAAAAAAAAAAAAHbWVtYmVycwAAAAADAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAAAAAABXN0YXJ0AAAAAAAABAAAAAAAAAAFbGltaXQAAAAAAAAEAAAAAQAAA+oAAAAT",
        "AAAAAAAAAAAAAAAIZGlzcGF0Y2gAAAADAAAAAAAAAAhzdXBwbGllcgAAABMAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAAMd2F5YmlsbF9oYXNoAAAD6AAAA+4AAAAgAAAAAA==",
        "AAAAAAAAAAAAAAAIaW5jcmVhc2UAAAADAAAAAAAAAAZtZW1iZXIAAAAAABMAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAALZXh0cmFfdW5pdHMAAAAABAAAAAEAAAAL",
        "AAAAAAAAAAAAAAAKY29tbWl0bWVudAAAAAAAAgAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAAAAAAZtZW1iZXIAAAAAABMAAAABAAAD6AAAB9AAAAAKQ29tbWl0bWVudAAA",
        "AAAAAAAAAEdTZW5kcyByb3VuZGluZyBkdXN0IChlc2Nyb3cgYmV5b25kIHdoYXQgaXMgc3RpbGwgb3dlZCkgdG8gdGhlIHRyZWFzdXJ5LgAAAAAKc3dlZXBfZHVzdAAAAAAAAQAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAQAAAAs=",
        "AAAAAAAAAEhPcmdhbml6ZXIgaWYgbm90aGluZyBpcyBjb21taXR0ZWQgeWV0OyBhZG1pbiBhbnkgdGltZSBiZWZvcmUgRGlzcGF0Y2hlZC4AAAALY2FuY2VsX3Bvb2wAAAAAAgAAAAAAAAAGY2FsbGVyAAAAAAATAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAA",
        "AAAAAAAAAAAAAAALY2xvc2VfZWFybHkAAAAAAgAAAAAAAAAJb3JnYW5pemVyAAAAAAAAEwAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAA==",
        "AAAAAAAAAAAAAAALY3JlYXRlX3Bvb2wAAAAABgAAAAAAAAAJb3JnYW5pemVyAAAAAAAAEwAAAAAAAAAFdGVybXMAAAAAAAfQAAAACVBvb2xUZXJtcwAAAAAAAAAAAAAIaHViX2hhc2gAAAPuAAAAIAAAAAAAAAAQb3JnYW5pemVyX2ZlZV9icAAAAAQAAAAAAAAAB2NsdXN0ZXIAAAAD6AAAABEAAAAAAAAADWZpbGxfZGVhZGxpbmUAAAAAAAAGAAAAAQAAAAY=",
        "AAAAAAAAACdQZXJtaXNzaW9ubGVzcyBhZnRlciB0aGUgYWNjZXB0IHdpbmRvdy4AAAAAC2ZhaWxfYWNjZXB0AAAAAAEAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAA=",
        "AAAAAAAAAAAAAAAMY2xhaW1fcmVmdW5kAAAAAgAAAAAAAAAGbWVtYmVyAAAAAAATAAAAAAAAAAdwb29sX2lkAAAAAAYAAAABAAAACw==",
        "AAAAAAAAAAAAAAAMY3VycmVudF90aWVyAAAAAQAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAQAAA+0AAAACAAAABAAAAAs=",
        "AAAAAAAAAEhQZXJtaXNzaW9ubGVzcyBib3VuZGVkIGJhdGNoIHB1c2ggb2YgcmVmdW5kcy4gUmV0dXJucyBtZW1iZXJzIHByb2Nlc3NlZC4AAAAMcHVzaF9yZWZ1bmRzAAAAAgAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAAAAAANtYXgAAAAABAAAAAEAAAAE",
        "AAAAAAAAAAAAAAAMcXVvdGVfY29tbWl0AAAAAgAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAAAAAAV1bml0cwAAAAAAAAQAAAABAAAACw==",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAEAAAAAAAAABmNvbmZpZwAAAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAANYXBwbHlfb3V0Y29tZQAAAAAAAAUAAAAAAAAABmNhbGxlcgAAAAAAEwAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAAAAAAZtZW1iZXIAAAAAA+gAAAATAAAAAAAAAAl0b19tZW1iZXIAAAAAAAALAAAAAAAAAAt0b19zdXBwbGllcgAAAAALAAAAAA==",
        "AAAAAAAAADdQZXJtaXNzaW9ubGVzcyBhZnRlciBgYWNjZXB0ZWRfYXQgKyBsZWFkX3RpbWUgKyBncmFjZWAuAAAAAA1mYWlsX2RlbGl2ZXJ5AAAAAAAAAQAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAA==",
        "AAAAAAAAAAAAAAANcmVmdW5kYWJsZV9vZgAAAAAAAAIAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAAGbWVtYmVyAAAAAAATAAAAAQAAAAs=",
        "AAAAAAAAAAAAAAAOY29uZmlybV9waWNrdXAAAAAAAAIAAAAAAAAABm1lbWJlcgAAAAAAEwAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAA==",
        "AAAAAAAAAEEoc3VwcGxpZXIsIHBsYXRmb3JtLCBvcmdhbml6ZXIsIHJlZnVuZHMpIGlmIHRoZSBwb29sIHNldHRsZWQgbm93LgAAAAAAAA5wYXlvdXRfcHJldmlldwAAAAAAAQAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAQAAA+0AAAAEAAAACwAAAAsAAAALAAAACw==",
        "AAAAAAAAAAAAAAAQY29uZmlybV9kZWFkbGluZQAAAAEAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAEAAAAG",
        "AAAAAAAAAAAAAAAQY29uZmlybV9kZWxpdmVyeQAAAAQAAAAAAAAACW9yZ2FuaXplcgAAAAAAABMAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAAOcmVjZWl2ZWRfdW5pdHMAAAAAAAQAAAAAAAAACGV2aWRlbmNlAAAD7gAAACAAAAAA",
        "AAAAAAAAAE5QZXJtaXNzaW9ubGVzcywgcGFnZWQuIE5lZWRlZCBvbmx5IGFmdGVyIGEgc2hvcnRmYWxsOyBmaWxscyBgYWxsb2NhdGVkX3VuaXRzYC4AAAAAABJhbGxvY2F0ZV9zaG9ydGZhbGwAAAAAAAIAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAAAAAADbWF4AAAAAAQAAAABAAAAAQ==",
        "AAAAAAAAAAAAAAASYWxsb2NhdGVkX3VuaXRzX29mAAAAAAACAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAAAAAABm1lbWJlcgAAAAAAEwAAAAEAAAAE",
        "AAAAAAAAAKlBbGxvd2VkIHdoaWxlIE9wZW4gYW5kIGJlZm9yZSB0aGUgbG9jaywgYW5kIG9ubHkgaWYgaXQgZG9lcyBub3QgbG93ZXIgdGhlIHBvb2wncyBwcmljZQp0aWVyIChBRFIgMDAwMSk6IHRoYXQga2VlcHMgZXZlcnkgbWVtYmVyJ3MgcGFpZCBhbW91bnQgYXQgb3IgYWJvdmUgdGhlIGZpbmFsIGJpbGwuAAAAAAAAE3dpdGhkcmF3X2NvbW1pdG1lbnQAAAAAAgAAAAAAAAAGbWVtYmVyAAAAAAATAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAA",
        "AAAAAAAAAJBDb2xsZWN0aXZlIGZhbGxiYWNrIHdoZW4gdGhlIG9yZ2FuaXplciBoYXMgbm90IGNvbmZpcm1lZCBpbiB0aW1lOiBtZW1iZXJzIGhvbGRpbmcgYXQKbGVhc3QgNTAlIG9mIHVuaXRzIGNhbiBjb25maXJtIHRoYXQgdGhlIGZ1bGwgb3JkZXIgYXJyaXZlZC4AAAAXbWVtYmVyX2NvbmZpcm1fZGVsaXZlcnkAAAAAAgAAAAAAAAAGbWVtYmVyAAAAAAATAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAA",
        "AAAAAQAAAAAAAAAAAAAABFBvb2wAAAAhAAAAAAAAAAthY2NlcHRlZF9hdAAAAAAGAAAAAAAAAAxhZHZhbmNlX3BhaWQAAAALAAAAAAAAAAlhbGxvY19jdW0AAAAAAAAEAAAAAAAAAAxhbGxvY19jdXJzb3IAAAAEAAAAAAAAAAphbGxvY19kb25lAAAAAAABAAAAAAAAAAdjbHVzdGVyAAAAA+gAAAARAAAAAAAAAA1jb25maXJtX3VuaXRzAAAAAAAABAAAAAAAAAAKY3JlYXRlZF9hdAAAAAAABgAAAAAAAAAMZGVsaXZlcmVkX2F0AAAABgAAAAAAAAANZGlzcGF0Y2hlZF9hdAAAAAAAAAYAAABARGlzcHV0ZSBvdXRjb21lcyBhbHJlYWR5IGVhcm1hcmtlZCBmb3IgbWVtYmVycyAocHJlLXNldHRsZW1lbnQpLgAAAAhkaXZlcnRlZAAAAAsAAAAuVG9rZW5zIHRoZSBjb250cmFjdCBzdGlsbCBob2xkcyBmb3IgdGhpcyBwb29sLgAAAAAADmVzY3Jvd19iYWxhbmNlAAAAAAALAAAAREV4dHJhIHJlZnVuZCBzaGFyZWQgcHJvLXJhdGEgYW1vbmcgYWxsIG1lbWJlcnMgKFJlZnVuZFBvb2wgb3V0Y29tZSkuAAAAEWV4dHJhX3JlZnVuZF9wb29sAAAAAAAACwAAAAAAAAANZmlsbF9kZWFkbGluZQAAAAAAAAYAAAAAAAAACWZpbGxlZF9hdAAAAAAAAAYAAAAyUHJpY2UgcGVyIHVuaXQgZml4ZWQgd2hlbiB0aGUgcG9vbCByZWFjaGVzIEZpbGxlZC4AAAAAAAtmaW5hbF9wcmljZQAAAAALAAAAAAAAAA1mcm96ZW5fYW1vdW50AAAAAAAACwAAAAAAAAAIaHViX2hhc2gAAAPuAAAAIAAAAAAAAAACaWQAAAAAAAYAAAA3TGVuZ3RoIG9mIHRoZSBtZW1iZXIgbGlzdCwgaW5jbHVkaW5nIHdpdGhkcmF3biBtZW1iZXJzLgAAAAAObGlzdGVkX21lbWJlcnMAAAAAAAQAAAAXTWVtYmVycyB3aXRoIHVuaXRzID4gMC4AAAAADG1lbWJlcl9jb3VudAAAAAQAAAAAAAAACW9yZ2FuaXplcgAAAAAAABMAAAAAAAAAEG9yZ2FuaXplcl9mZWVfYnAAAAAEAAAARlRvdGFsIG93ZWQgYmFjayB0byBtZW1iZXJzIG9uY2UgdGhlIHBvb2wgaXMgZmluYWwgKHJlZnVuZHMgKyBjcmVkaXRzKS4AAAAAAApvd2VkX3RvdGFsAAAAAAALAAAAAAAAAAxwaWNrZWRfdW5pdHMAAAAEAAAAAAAAAA9wbGF0Zm9ybV9mZWVfYnAAAAAABAAAAAAAAAAOcmVjZWl2ZWRfdW5pdHMAAAAAAAQAAAAAAAAADXJlZnVuZF9jdXJzb3IAAAAAAAAEAAAAAAAAAAxyZWZ1bmRzX3BhaWQAAAALAAAAAAAAAAVzdGF0ZQAAAAAAB9AAAAAJUG9vbFN0YXRlAAAAAAAAAAAAAAV0ZXJtcwAAAAAAB9AAAAAJUG9vbFRlcm1zAAAAAAAAQlN1bSBvZiBhbGwgbWVtYmVyIHBheW1lbnRzIChuZXZlciBkZWNyZWFzZXMgZXhjZXB0IG9uIHdpdGhkcmF3YWwpLgAAAAAACnRvdGFsX3BhaWQAAAAAAAsAAAAAAAAAC3RvdGFsX3VuaXRzAAAAAAQ=",
        "AAAAAQAAAAAAAAAAAAAABFRpZXIAAAACAAAAAAAAAAltaW5fdW5pdHMAAAAAAAAEAAAAAAAAAAp1bml0X3ByaWNlAAAAAAAL",
        "AAAAAgAAAAAAAAAAAAAACVBvb2xTdGF0ZQAAAAAAAAkAAAAAAAAAAAAAAARPcGVuAAAAAAAAAAAAAAAGRmlsbGVkAAAAAAAAAAAAAAAAAAhBY2NlcHRlZAAAAAAAAAAAAAAACkRpc3BhdGNoZWQAAAAAAAAAAAAAAAAACURlbGl2ZXJlZAAAAAAAAAAAAAAAAAAAB1NldHRsZWQAAAAAAAAAAAAAAAAHRXhwaXJlZAAAAAAAAAAAAAAAAAZGYWlsZWQAAAAAAAAAAAAAAAAACUNhbmNlbGxlZAAAAA==",
        "AAAAAQAAAAAAAAAAAAAACVBvb2xUZXJtcwAAAAAAAAoAAAAAAAAACGNhdGVnb3J5AAAAEQAAAAAAAAAObGVhZF90aW1lX3NlY3MAAAAAAAYAAAAAAAAADm1heF9wZXJfbWVtYmVyAAAAAAAEAAAAAAAAAAltYXhfdW5pdHMAAAAAAAAEAAAAAAAAAANtb3EAAAAABAAAAAAAAAAKb2ZmZXJfaGFzaAAAAAAD7gAAACAAAAAAAAAACnBlcmlzaGFibGUAAAAAAAEAAAAAAAAACHN1cHBsaWVyAAAAEwAAAAAAAAAFdGllcnMAAAAAAAPqAAAH0AAAAARUaWVyAAAAAAAAAA91bml0X2xhYmVsX2hhc2gAAAAD7gAAACA=",
        "AAAAAQAAAAAAAAAAAAAACkNvbW1pdG1lbnQAAAAAAAkAAAAAAAAAD2FsbG9jYXRlZF91bml0cwAAAAAEAAAAAAAAAAxjb21taXR0ZWRfYXQAAAAGAAAAAAAAABJkZWxpdmVyeV9jb25maXJtZWQAAAAAAAEAAAAjRXh0cmEgY3JlZGl0IGZyb20gZGlzcHV0ZSBvdXRjb21lcy4AAAAADGV4dHJhX2NyZWRpdAAAAAsAAAAAAAAABm1lbWJlcgAAAAAAEwAAAAAAAAAEcGFpZAAAAAsAAAAAAAAACXBpY2tlZF91cAAAAAAAAAEAAAAzVG90YWwgcmVmdW5kIGFtb3VudCBhbHJlYWR5IHBhaWQgb3V0IHRvIHRoZSBtZW1iZXIuAAAAAA5yZWZ1bmRfY2xhaW1lZAAAAAAACwAAAAAAAAAFdW5pdHMAAAAAAAAE" ]),
      options
    )
  }
  public readonly fromJSON = {
    pool: this.txFromJSON<Pool>,
        close: this.txFromJSON<null>,
        accept: this.txFromJSON<null>,
        commit: this.txFromJSON<i128>,
        freeze: this.txFromJSON<null>,
        reject: this.txFromJSON<null>,
        settle: this.txFromJSON<null>,
        members: this.txFromJSON<Array<string>>,
        dispatch: this.txFromJSON<null>,
        increase: this.txFromJSON<i128>,
        commitment: this.txFromJSON<Option<Commitment>>,
        sweep_dust: this.txFromJSON<i128>,
        cancel_pool: this.txFromJSON<null>,
        close_early: this.txFromJSON<null>,
        create_pool: this.txFromJSON<u64>,
        fail_accept: this.txFromJSON<null>,
        claim_refund: this.txFromJSON<i128>,
        current_tier: this.txFromJSON<readonly [u32, i128]>,
        push_refunds: this.txFromJSON<u32>,
        quote_commit: this.txFromJSON<i128>,
        apply_outcome: this.txFromJSON<null>,
        fail_delivery: this.txFromJSON<null>,
        refundable_of: this.txFromJSON<i128>,
        confirm_pickup: this.txFromJSON<null>,
        payout_preview: this.txFromJSON<readonly [i128, i128, i128, i128]>,
        confirm_deadline: this.txFromJSON<u64>,
        confirm_delivery: this.txFromJSON<null>,
        allocate_shortfall: this.txFromJSON<boolean>,
        allocated_units_of: this.txFromJSON<u32>,
        withdraw_commitment: this.txFromJSON<null>,
        member_confirm_delivery: this.txFromJSON<null>
  }
}