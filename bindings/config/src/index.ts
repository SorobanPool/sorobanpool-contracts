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
  100: {message:"NotAdmin"},
  101: {message:"NotAdminOrGuardian"},
  102: {message:"ParamsInvalid"},
  103: {message:"ParamsExceedHardCap"},
  104: {message:"AddressNotSet"},
  105: {message:"NoGuardian"},
  106: {message:"NoAdminProposal"},
  107: {message:"NotProposedAdmin"},
  108: {message:"TooManyArbiters"},
  109: {message:"TooManyAttestors"},
  110: {message:"TooManyCategories"}
}


/**
 * Read-only limits used by `set_params`. Keep this in sync with `caps`.
 */
export interface HardCaps {
  arbitration_sla_secs: u64;
  basis_points: u32;
  dispute_deposit_bp: u32;
  max_fill_window_secs: u64;
  max_organizer_fee_bp: u32;
  organizer_tier_count: u32;
  platform_fee_bp: u32;
  supplier_advance_bp: u32;
  supplier_advance_tier_count: u32;
  supplier_tier_count: u32;
  trader_tier_count: u32;
}


/**
 * Returned by `snapshot()`: params plus a flag for each supported pause scope.
 */
export interface ConfigSnapshot {
  params: Params;
  paused_accept: boolean;
  paused_all: boolean;
  paused_commit: boolean;
  paused_create: boolean;
}


export interface Params {
  accept_window_secs: u64;
  advance_enabled: boolean;
  arbitration_sla_secs: u64;
  bond_withdraw_cooldown_secs: u64;
  confirm_window_secs: u64;
  delivery_grace_secs: u64;
  dispute_deposit_bp: u32;
  dispute_deposit_min: i128;
  early_release_weight_bp: u32;
  max_fill_window_secs: u64;
  max_member_share_bp: u32;
  max_members_per_pool: u32;
  max_organizer_fee_bp: u32;
  organizer_confirm_window_secs: u64;
  organizer_tier_caps: Array<i128>;
  perishable_confirm_window_secs: u64;
  platform_fee_bp: u32;
  supplier_advance_bp: Array<u32>;
  supplier_tier_caps: Array<i128>;
  trader_tier_caps: Array<i128>;
  withdraw_lock_secs: u64;
}

export interface Client {
  /**
   * Construct and simulate a usdc transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  usdc: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  admin: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a pause transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Scopes: "create", "commit", "accept", "all". Admin or guardian.
   * Pausing never blocks refunds, expiry, failures, dispute timeouts or settlement.
   */
  pause: ({caller, scope}: {caller: string, scope: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a unpause transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  unpause: ({scope}: {scope: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  upgrade: ({wasm_hash}: {wasm_hash: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a arbiters transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  arbiters: (options?: MethodOptions) => Promise<AssembledTransaction<Array<string>>>

  /**
   * Construct and simulate a snapshot transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Read-only: returns params and every pause-scope flag in one RPC call.
   */
  snapshot: (options?: MethodOptions) => Promise<AssembledTransaction<ConfigSnapshot>>

  /**
   * Construct and simulate a treasury transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  treasury: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a attestors transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  attestors: (options?: MethodOptions) => Promise<AssembledTransaction<Array<string>>>

  /**
   * Construct and simulate a is_paused transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_paused: ({scope}: {scope: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a categories transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  categories: (options?: MethodOptions) => Promise<AssembledTransaction<Array<string>>>

  /**
   * Construct and simulate a get_params transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_params: (options?: MethodOptions) => Promise<AssembledTransaction<Params>>

  /**
   * Construct and simulate a is_arbiter transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_arbiter: ({a}: {a: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a set_params transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_params: ({params}: {params: Params}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_address transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_address: ({key}: {key: string}, options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a is_attestor transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_attestor: ({a}: {a: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a set_address transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_address: ({key, addr}: {key: string, addr: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_arbiter transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_arbiter: ({a, enabled}: {a: string, enabled: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a accept_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  accept_admin: (options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_attestor transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_attestor: ({a, enabled}: {a: string, enabled: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_category transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_category: ({cat, allowed}: {cat: string, allowed: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a set_guardian transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_guardian: ({g}: {g: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a get_hard_caps transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_hard_caps: (options?: MethodOptions) => Promise<AssembledTransaction<HardCaps>>

  /**
   * Construct and simulate a propose_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  propose_admin: ({new_admin}: {new_admin: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a proposed_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  proposed_admin: (options?: MethodOptions) => Promise<AssembledTransaction<Option<string>>>

  /**
   * Construct and simulate a is_category_allowed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_category_allowed: ({cat}: {cat: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a cancel_admin_proposal transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  cancel_admin_proposal: (options?: MethodOptions) => Promise<AssembledTransaction<null>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
        /** Constructor/Initialization Args for the contract's `__constructor` method */
        {admin, usdc, treasury}: {admin: string, usdc: string, treasury: string},
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
    return ContractClient.deploy({admin, usdc, treasury}, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAACwAAAAAAAAAITm90QWRtaW4AAABkAAAAAAAAABJOb3RBZG1pbk9yR3VhcmRpYW4AAAAAAGUAAAAAAAAADVBhcmFtc0ludmFsaWQAAAAAAABmAAAAAAAAABNQYXJhbXNFeGNlZWRIYXJkQ2FwAAAAAGcAAAAAAAAADUFkZHJlc3NOb3RTZXQAAAAAAABoAAAAAAAAAApOb0d1YXJkaWFuAAAAAABpAAAAAAAAAA9Ob0FkbWluUHJvcG9zYWwAAAAAagAAAAAAAAAQTm90UHJvcG9zZWRBZG1pbgAAAGsAAAAAAAAAD1Rvb01hbnlBcmJpdGVycwAAAABsAAAAAAAAABBUb29NYW55QXR0ZXN0b3JzAAAAbQAAAAAAAAARVG9vTWFueUNhdGVnb3JpZXMAAAAAAABu",
        "AAAAAQAAAEVSZWFkLW9ubHkgbGltaXRzIHVzZWQgYnkgYHNldF9wYXJhbXNgLiBLZWVwIHRoaXMgaW4gc3luYyB3aXRoIGBjYXBzYC4AAAAAAAAAAAAACEhhcmRDYXBzAAAACwAAAAAAAAAUYXJiaXRyYXRpb25fc2xhX3NlY3MAAAAGAAAAAAAAAAxiYXNpc19wb2ludHMAAAAEAAAAAAAAABJkaXNwdXRlX2RlcG9zaXRfYnAAAAAAAAQAAAAAAAAAFG1heF9maWxsX3dpbmRvd19zZWNzAAAABgAAAAAAAAAUbWF4X29yZ2FuaXplcl9mZWVfYnAAAAAEAAAAAAAAABRvcmdhbml6ZXJfdGllcl9jb3VudAAAAAQAAAAAAAAAD3BsYXRmb3JtX2ZlZV9icAAAAAAEAAAAAAAAABNzdXBwbGllcl9hZHZhbmNlX2JwAAAAAAQAAAAAAAAAG3N1cHBsaWVyX2FkdmFuY2VfdGllcl9jb3VudAAAAAAEAAAAAAAAABNzdXBwbGllcl90aWVyX2NvdW50AAAAAAQAAAAAAAAAEXRyYWRlcl90aWVyX2NvdW50AAAAAAAABA==",
        "AAAAAQAAAExSZXR1cm5lZCBieSBgc25hcHNob3QoKWA6IHBhcmFtcyBwbHVzIGEgZmxhZyBmb3IgZWFjaCBzdXBwb3J0ZWQgcGF1c2Ugc2NvcGUuAAAAAAAAAA5Db25maWdTbmFwc2hvdAAAAAAABQAAAAAAAAAGcGFyYW1zAAAAAAfQAAAABlBhcmFtcwAAAAAAAAAAAA1wYXVzZWRfYWNjZXB0AAAAAAAAAQAAAAAAAAAKcGF1c2VkX2FsbAAAAAAAAQAAAAAAAAANcGF1c2VkX2NvbW1pdAAAAAAAAAEAAAAAAAAADXBhdXNlZF9jcmVhdGUAAAAAAAAB",
        "AAAAAAAAAAAAAAAEdXNkYwAAAAAAAAABAAAAEw==",
        "AAAAAAAAAAAAAAAFYWRtaW4AAAAAAAAAAAAAAQAAABM=",
        "AAAAAAAAAI9TY29wZXM6ICJjcmVhdGUiLCAiY29tbWl0IiwgImFjY2VwdCIsICJhbGwiLiBBZG1pbiBvciBndWFyZGlhbi4KUGF1c2luZyBuZXZlciBibG9ja3MgcmVmdW5kcywgZXhwaXJ5LCBmYWlsdXJlcywgZGlzcHV0ZSB0aW1lb3V0cyBvciBzZXR0bGVtZW50LgAAAAAFcGF1c2UAAAAAAAACAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAABXNjb3BlAAAAAAAAEQAAAAA=",
        "AAAAAAAAAAAAAAAHdW5wYXVzZQAAAAABAAAAAAAAAAVzY29wZQAAAAAAABEAAAAA",
        "AAAAAAAAAAAAAAAHdXBncmFkZQAAAAABAAAAAAAAAAl3YXNtX2hhc2gAAAAAAAPuAAAAIAAAAAA=",
        "AAAAAAAAAAAAAAAIYXJiaXRlcnMAAAAAAAAAAQAAA+oAAAAT",
        "AAAAAAAAAEVSZWFkLW9ubHk6IHJldHVybnMgcGFyYW1zIGFuZCBldmVyeSBwYXVzZS1zY29wZSBmbGFnIGluIG9uZSBSUEMgY2FsbC4AAAAAAAAIc25hcHNob3QAAAAAAAAAAQAAB9AAAAAOQ29uZmlnU25hcHNob3QAAA==",
        "AAAAAAAAAAAAAAAIdHJlYXN1cnkAAAAAAAAAAQAAABM=",
        "AAAAAAAAAAAAAAAJYXR0ZXN0b3JzAAAAAAAAAAAAAAEAAAPqAAAAEw==",
        "AAAAAAAAAAAAAAAJaXNfcGF1c2VkAAAAAAAAAQAAAAAAAAAFc2NvcGUAAAAAAAARAAAAAQAAAAE=",
        "AAAAAAAAAAAAAAAKY2F0ZWdvcmllcwAAAAAAAAAAAAEAAAPqAAAAEQ==",
        "AAAAAAAAAAAAAAAKZ2V0X3BhcmFtcwAAAAAAAAAAAAEAAAfQAAAABlBhcmFtcwAA",
        "AAAAAAAAAAAAAAAKaXNfYXJiaXRlcgAAAAAAAQAAAAAAAAABYQAAAAAAABMAAAABAAAAAQ==",
        "AAAAAAAAAAAAAAAKc2V0X3BhcmFtcwAAAAAAAQAAAAAAAAAGcGFyYW1zAAAAAAfQAAAABlBhcmFtcwAAAAAAAA==",
        "AAAAAAAAAAAAAAALZ2V0X2FkZHJlc3MAAAAAAQAAAAAAAAADa2V5AAAAABEAAAABAAAAEw==",
        "AAAAAAAAAAAAAAALaXNfYXR0ZXN0b3IAAAAAAQAAAAAAAAABYQAAAAAAABMAAAABAAAAAQ==",
        "AAAAAAAAAAAAAAALc2V0X2FkZHJlc3MAAAAAAgAAAAAAAAADa2V5AAAAABEAAAAAAAAABGFkZHIAAAATAAAAAA==",
        "AAAAAAAAAAAAAAALc2V0X2FyYml0ZXIAAAAAAgAAAAAAAAABYQAAAAAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAA=",
        "AAAAAAAAAAAAAAAMYWNjZXB0X2FkbWluAAAAAAAAAAA=",
        "AAAAAAAAAAAAAAAMc2V0X2F0dGVzdG9yAAAAAgAAAAAAAAABYQAAAAAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAA=",
        "AAAAAAAAAAAAAAAMc2V0X2NhdGVnb3J5AAAAAgAAAAAAAAADY2F0AAAAABEAAAAAAAAAB2FsbG93ZWQAAAAAAQAAAAA=",
        "AAAAAAAAAAAAAAAMc2V0X2d1YXJkaWFuAAAAAQAAAAAAAAABZwAAAAAAABMAAAAA",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAMAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAEdXNkYwAAABMAAAAAAAAACHRyZWFzdXJ5AAAAEwAAAAA=",
        "AAAAAAAAAAAAAAANZ2V0X2hhcmRfY2FwcwAAAAAAAAAAAAABAAAH0AAAAAhIYXJkQ2Fwcw==",
        "AAAAAAAAAAAAAAANcHJvcG9zZV9hZG1pbgAAAAAAAAEAAAAAAAAACW5ld19hZG1pbgAAAAAAABMAAAAA",
        "AAAAAAAAAAAAAAAOcHJvcG9zZWRfYWRtaW4AAAAAAAAAAAABAAAD6AAAABM=",
        "AAAAAAAAAAAAAAATaXNfY2F0ZWdvcnlfYWxsb3dlZAAAAAABAAAAAAAAAANjYXQAAAAAEQAAAAEAAAAB",
        "AAAAAAAAAAAAAAAVY2FuY2VsX2FkbWluX3Byb3Bvc2FsAAAAAAAAAAAAAAA=",
        "AAAAAQAAAAAAAAAAAAAABlBhcmFtcwAAAAAAFQAAAAAAAAASYWNjZXB0X3dpbmRvd19zZWNzAAAAAAAGAAAAAAAAAA9hZHZhbmNlX2VuYWJsZWQAAAAAAQAAAAAAAAAUYXJiaXRyYXRpb25fc2xhX3NlY3MAAAAGAAAAAAAAABtib25kX3dpdGhkcmF3X2Nvb2xkb3duX3NlY3MAAAAABgAAAAAAAAATY29uZmlybV93aW5kb3dfc2VjcwAAAAAGAAAAAAAAABNkZWxpdmVyeV9ncmFjZV9zZWNzAAAAAAYAAAAAAAAAEmRpc3B1dGVfZGVwb3NpdF9icAAAAAAABAAAAAAAAAATZGlzcHV0ZV9kZXBvc2l0X21pbgAAAAALAAAAAAAAABdlYXJseV9yZWxlYXNlX3dlaWdodF9icAAAAAAEAAAAAAAAABRtYXhfZmlsbF93aW5kb3dfc2VjcwAAAAYAAAAAAAAAE21heF9tZW1iZXJfc2hhcmVfYnAAAAAABAAAAAAAAAAUbWF4X21lbWJlcnNfcGVyX3Bvb2wAAAAEAAAAAAAAABRtYXhfb3JnYW5pemVyX2ZlZV9icAAAAAQAAAAAAAAAHW9yZ2FuaXplcl9jb25maXJtX3dpbmRvd19zZWNzAAAAAAAABgAAAAAAAAATb3JnYW5pemVyX3RpZXJfY2FwcwAAAAPqAAAACwAAAAAAAAAecGVyaXNoYWJsZV9jb25maXJtX3dpbmRvd19zZWNzAAAAAAAGAAAAAAAAAA9wbGF0Zm9ybV9mZWVfYnAAAAAABAAAAAAAAAATc3VwcGxpZXJfYWR2YW5jZV9icAAAAAPqAAAABAAAAAAAAAASc3VwcGxpZXJfdGllcl9jYXBzAAAAAAPqAAAACwAAAAAAAAAQdHJhZGVyX3RpZXJfY2FwcwAAA+oAAAALAAAAAAAAABJ3aXRoZHJhd19sb2NrX3NlY3MAAAAAAAY=" ]),
      options
    )
  }
  public readonly fromJSON = {
    usdc: this.txFromJSON<string>,
        admin: this.txFromJSON<string>,
        pause: this.txFromJSON<null>,
        unpause: this.txFromJSON<null>,
        upgrade: this.txFromJSON<null>,
        arbiters: this.txFromJSON<Array<string>>,
        snapshot: this.txFromJSON<ConfigSnapshot>,
        treasury: this.txFromJSON<string>,
        attestors: this.txFromJSON<Array<string>>,
        is_paused: this.txFromJSON<boolean>,
        categories: this.txFromJSON<Array<string>>,
        get_params: this.txFromJSON<Params>,
        is_arbiter: this.txFromJSON<boolean>,
        set_params: this.txFromJSON<null>,
        get_address: this.txFromJSON<string>,
        is_attestor: this.txFromJSON<boolean>,
        set_address: this.txFromJSON<null>,
        set_arbiter: this.txFromJSON<null>,
        accept_admin: this.txFromJSON<null>,
        set_attestor: this.txFromJSON<null>,
        set_category: this.txFromJSON<null>,
        set_guardian: this.txFromJSON<null>,
        get_hard_caps: this.txFromJSON<HardCaps>,
        propose_admin: this.txFromJSON<null>,
        proposed_admin: this.txFromJSON<Option<string>>,
        is_category_allowed: this.txFromJSON<boolean>,
        cancel_admin_proposal: this.txFromJSON<null>
  }
}