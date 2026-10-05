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
  105: {message:"NoGuardian"}
}


export interface Params {
  accept_window_secs: u64;
  advance_enabled: boolean;
  arbitration_sla_secs: u64;
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
   * Construct and simulate a treasury transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  treasury: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a is_paused transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_paused: ({scope}: {scope: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

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
   * Construct and simulate a is_category_allowed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_category_allowed: ({cat}: {cat: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

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
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAABgAAAAAAAAAITm90QWRtaW4AAABkAAAAAAAAABJOb3RBZG1pbk9yR3VhcmRpYW4AAAAAAGUAAAAAAAAADVBhcmFtc0ludmFsaWQAAAAAAABmAAAAAAAAABNQYXJhbXNFeGNlZWRIYXJkQ2FwAAAAAGcAAAAAAAAADUFkZHJlc3NOb3RTZXQAAAAAAABoAAAAAAAAAApOb0d1YXJkaWFuAAAAAABp",
        "AAAAAAAAAAAAAAAEdXNkYwAAAAAAAAABAAAAEw==",
        "AAAAAAAAAAAAAAAFYWRtaW4AAAAAAAAAAAAAAQAAABM=",
        "AAAAAAAAAI9TY29wZXM6ICJjcmVhdGUiLCAiY29tbWl0IiwgImFjY2VwdCIsICJhbGwiLiBBZG1pbiBvciBndWFyZGlhbi4KUGF1c2luZyBuZXZlciBibG9ja3MgcmVmdW5kcywgZXhwaXJ5LCBmYWlsdXJlcywgZGlzcHV0ZSB0aW1lb3V0cyBvciBzZXR0bGVtZW50LgAAAAAFcGF1c2UAAAAAAAACAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAABXNjb3BlAAAAAAAAEQAAAAA=",
        "AAAAAAAAAAAAAAAHdW5wYXVzZQAAAAABAAAAAAAAAAVzY29wZQAAAAAAABEAAAAA",
        "AAAAAAAAAAAAAAAHdXBncmFkZQAAAAABAAAAAAAAAAl3YXNtX2hhc2gAAAAAAAPuAAAAIAAAAAA=",
        "AAAAAAAAAAAAAAAIdHJlYXN1cnkAAAAAAAAAAQAAABM=",
        "AAAAAAAAAAAAAAAJaXNfcGF1c2VkAAAAAAAAAQAAAAAAAAAFc2NvcGUAAAAAAAARAAAAAQAAAAE=",
        "AAAAAAAAAAAAAAAKZ2V0X3BhcmFtcwAAAAAAAAAAAAEAAAfQAAAABlBhcmFtcwAA",
        "AAAAAAAAAAAAAAAKaXNfYXJiaXRlcgAAAAAAAQAAAAAAAAABYQAAAAAAABMAAAABAAAAAQ==",
        "AAAAAAAAAAAAAAAKc2V0X3BhcmFtcwAAAAAAAQAAAAAAAAAGcGFyYW1zAAAAAAfQAAAABlBhcmFtcwAAAAAAAA==",
        "AAAAAAAAAAAAAAALZ2V0X2FkZHJlc3MAAAAAAQAAAAAAAAADa2V5AAAAABEAAAABAAAAEw==",
        "AAAAAAAAAAAAAAALaXNfYXR0ZXN0b3IAAAAAAQAAAAAAAAABYQAAAAAAABMAAAABAAAAAQ==",
        "AAAAAAAAAAAAAAALc2V0X2FkZHJlc3MAAAAAAgAAAAAAAAADa2V5AAAAABEAAAAAAAAABGFkZHIAAAATAAAAAA==",
        "AAAAAAAAAAAAAAALc2V0X2FyYml0ZXIAAAAAAgAAAAAAAAABYQAAAAAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAA=",
        "AAAAAAAAAAAAAAAMc2V0X2F0dGVzdG9yAAAAAgAAAAAAAAABYQAAAAAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAA=",
        "AAAAAAAAAAAAAAAMc2V0X2NhdGVnb3J5AAAAAgAAAAAAAAADY2F0AAAAABEAAAAAAAAAB2FsbG93ZWQAAAAAAQAAAAA=",
        "AAAAAAAAAAAAAAAMc2V0X2d1YXJkaWFuAAAAAQAAAAAAAAABZwAAAAAAABMAAAAA",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAMAAAAAAAAABWFkbWluAAAAAAAAEwAAAAAAAAAEdXNkYwAAABMAAAAAAAAACHRyZWFzdXJ5AAAAEwAAAAA=",
        "AAAAAAAAAAAAAAATaXNfY2F0ZWdvcnlfYWxsb3dlZAAAAAABAAAAAAAAAANjYXQAAAAAEQAAAAEAAAAB",
        "AAAAAQAAAAAAAAAAAAAABlBhcmFtcwAAAAAAFAAAAAAAAAASYWNjZXB0X3dpbmRvd19zZWNzAAAAAAAGAAAAAAAAAA9hZHZhbmNlX2VuYWJsZWQAAAAAAQAAAAAAAAAUYXJiaXRyYXRpb25fc2xhX3NlY3MAAAAGAAAAAAAAABNjb25maXJtX3dpbmRvd19zZWNzAAAAAAYAAAAAAAAAE2RlbGl2ZXJ5X2dyYWNlX3NlY3MAAAAABgAAAAAAAAASZGlzcHV0ZV9kZXBvc2l0X2JwAAAAAAAEAAAAAAAAABNkaXNwdXRlX2RlcG9zaXRfbWluAAAAAAsAAAAAAAAAF2Vhcmx5X3JlbGVhc2Vfd2VpZ2h0X2JwAAAAAAQAAAAAAAAAFG1heF9maWxsX3dpbmRvd19zZWNzAAAABgAAAAAAAAATbWF4X21lbWJlcl9zaGFyZV9icAAAAAAEAAAAAAAAABRtYXhfbWVtYmVyc19wZXJfcG9vbAAAAAQAAAAAAAAAFG1heF9vcmdhbml6ZXJfZmVlX2JwAAAABAAAAAAAAAAdb3JnYW5pemVyX2NvbmZpcm1fd2luZG93X3NlY3MAAAAAAAAGAAAAAAAAABNvcmdhbml6ZXJfdGllcl9jYXBzAAAAA+oAAAALAAAAAAAAAB5wZXJpc2hhYmxlX2NvbmZpcm1fd2luZG93X3NlY3MAAAAAAAYAAAAAAAAAD3BsYXRmb3JtX2ZlZV9icAAAAAAEAAAAAAAAABNzdXBwbGllcl9hZHZhbmNlX2JwAAAAA+oAAAAEAAAAAAAAABJzdXBwbGllcl90aWVyX2NhcHMAAAAAA+oAAAALAAAAAAAAABB0cmFkZXJfdGllcl9jYXBzAAAD6gAAAAsAAAAAAAAAEndpdGhkcmF3X2xvY2tfc2VjcwAAAAAABg==" ]),
      options
    )
  }
  public readonly fromJSON = {
    usdc: this.txFromJSON<string>,
        admin: this.txFromJSON<string>,
        pause: this.txFromJSON<null>,
        unpause: this.txFromJSON<null>,
        upgrade: this.txFromJSON<null>,
        treasury: this.txFromJSON<string>,
        is_paused: this.txFromJSON<boolean>,
        get_params: this.txFromJSON<Params>,
        is_arbiter: this.txFromJSON<boolean>,
        set_params: this.txFromJSON<null>,
        get_address: this.txFromJSON<string>,
        is_attestor: this.txFromJSON<boolean>,
        set_address: this.txFromJSON<null>,
        set_arbiter: this.txFromJSON<null>,
        set_attestor: this.txFromJSON<null>,
        set_category: this.txFromJSON<null>,
        set_guardian: this.txFromJSON<null>,
        is_category_allowed: this.txFromJSON<boolean>
  }
}