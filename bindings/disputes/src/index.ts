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
  400: {message:"NotInitialised"},
  401: {message:"NotAParty"},
  402: {message:"BadClaim"},
  403: {message:"NotFound"},
  404: {message:"AlreadyResolved"},
  405: {message:"NotArbiter"},
  406: {message:"ArbiterConflict"},
  407: {message:"SlaPassed"},
  408: {message:"SlaNotReached"},
  409: {message:"BadOutcome"},
  410: {message:"TooMuchEvidence"},
  411: {message:"Overflow"}
}


export interface Dispute {
  claimed_amount: i128;
  claimed_units: u32;
  deposit: i128;
  evidence: Array<Buffer>;
  id: u64;
  opened_at: u64;
  opener: string;
  /**
 * Empty until resolved, then exactly one element (Option<Outcome> is not supported by the SDK).
 */
outcome: Array<Outcome>;
  pool_id: u64;
  reason: DisputeReason;
  resolved: boolean;
}

export type Outcome = {tag: "ReleaseToSupplier", values: void} | {tag: "RefundMember", values: readonly [u32]} | {tag: "Split", values: readonly [u32]} | {tag: "RefundPool", values: void};

export type DisputeReason = {tag: "Short", values: void} | {tag: "WrongItem", values: void} | {tag: "Damaged", values: void} | {tag: "Quality", values: void} | {tag: "NotDelivered", values: void} | {tag: "Other", values: void};

export interface Client {
  /**
   * Construct and simulate a open transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  open: ({opener, pool_id, reason, claimed_units, evidence}: {opener: string, pool_id: u64, reason: DisputeReason, claimed_units: u32, evidence: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<u64>>

  /**
   * Construct and simulate a dispute transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  dispute: ({id}: {id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<Dispute>>

  /**
   * Construct and simulate a resolve transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  resolve: ({arbiter, dispute_id, outcome, reasoning_hash}: {arbiter: string, dispute_id: u64, outcome: Outcome, reasoning_hash: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a timeout transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Permissionless once the arbitration SLA has passed: the frozen amount goes to the disputer.
   */
  timeout: ({dispute_id}: {dispute_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a add_evidence transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  add_evidence: ({party, dispute_id, evidence}: {party: string, dispute_id: u64, evidence: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a open_disputes_of transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  open_disputes_of: ({pool_id}: {pool_id: u64}, options?: MethodOptions) => Promise<AssembledTransaction<Array<u64>>>

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
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAADAAAAAAAAAAOTm90SW5pdGlhbGlzZWQAAAAAAZAAAAAAAAAACU5vdEFQYXJ0eQAAAAAAAZEAAAAAAAAACEJhZENsYWltAAABkgAAAAAAAAAITm90Rm91bmQAAAGTAAAAAAAAAA9BbHJlYWR5UmVzb2x2ZWQAAAABlAAAAAAAAAAKTm90QXJiaXRlcgAAAAABlQAAAAAAAAAPQXJiaXRlckNvbmZsaWN0AAAAAZYAAAAAAAAACVNsYVBhc3NlZAAAAAAAAZcAAAAAAAAADVNsYU5vdFJlYWNoZWQAAAAAAAGYAAAAAAAAAApCYWRPdXRjb21lAAAAAAGZAAAAAAAAAA9Ub29NdWNoRXZpZGVuY2UAAAABmgAAAAAAAAAIT3ZlcmZsb3cAAAGb",
        "AAAAAAAAAAAAAAAEb3BlbgAAAAUAAAAAAAAABm9wZW5lcgAAAAAAEwAAAAAAAAAHcG9vbF9pZAAAAAAGAAAAAAAAAAZyZWFzb24AAAAAB9AAAAANRGlzcHV0ZVJlYXNvbgAAAAAAAAAAAAANY2xhaW1lZF91bml0cwAAAAAAAAQAAAAAAAAACGV2aWRlbmNlAAAD7gAAACAAAAABAAAABg==",
        "AAAAAAAAAAAAAAAHZGlzcHV0ZQAAAAABAAAAAAAAAAJpZAAAAAAABgAAAAEAAAfQAAAAB0Rpc3B1dGUA",
        "AAAAAAAAAAAAAAAHcmVzb2x2ZQAAAAAEAAAAAAAAAAdhcmJpdGVyAAAAABMAAAAAAAAACmRpc3B1dGVfaWQAAAAAAAYAAAAAAAAAB291dGNvbWUAAAAH0AAAAAdPdXRjb21lAAAAAAAAAAAOcmVhc29uaW5nX2hhc2gAAAAAA+4AAAAgAAAAAA==",
        "AAAAAAAAAFtQZXJtaXNzaW9ubGVzcyBvbmNlIHRoZSBhcmJpdHJhdGlvbiBTTEEgaGFzIHBhc3NlZDogdGhlIGZyb3plbiBhbW91bnQgZ29lcyB0byB0aGUgZGlzcHV0ZXIuAAAAAAd0aW1lb3V0AAAAAAEAAAAAAAAACmRpc3B1dGVfaWQAAAAAAAYAAAAA",
        "AAAAAAAAAAAAAAAMYWRkX2V2aWRlbmNlAAAAAwAAAAAAAAAFcGFydHkAAAAAAAATAAAAAAAAAApkaXNwdXRlX2lkAAAAAAAGAAAAAAAAAAhldmlkZW5jZQAAA+4AAAAgAAAAAA==",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAEAAAAAAAAABmNvbmZpZwAAAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAAQb3Blbl9kaXNwdXRlc19vZgAAAAEAAAAAAAAAB3Bvb2xfaWQAAAAABgAAAAEAAAPqAAAABg==",
        "AAAAAQAAAAAAAAAAAAAAB0Rpc3B1dGUAAAAACwAAAAAAAAAOY2xhaW1lZF9hbW91bnQAAAAAAAsAAAAAAAAADWNsYWltZWRfdW5pdHMAAAAAAAAEAAAAAAAAAAdkZXBvc2l0AAAAAAsAAAAAAAAACGV2aWRlbmNlAAAD6gAAA+4AAAAgAAAAAAAAAAJpZAAAAAAABgAAAAAAAAAJb3BlbmVkX2F0AAAAAAAABgAAAAAAAAAGb3BlbmVyAAAAAAATAAAAXUVtcHR5IHVudGlsIHJlc29sdmVkLCB0aGVuIGV4YWN0bHkgb25lIGVsZW1lbnQgKE9wdGlvbjxPdXRjb21lPiBpcyBub3Qgc3VwcG9ydGVkIGJ5IHRoZSBTREspLgAAAAAAAAdvdXRjb21lAAAAA+oAAAfQAAAAB091dGNvbWUAAAAAAAAAAAdwb29sX2lkAAAAAAYAAAAAAAAABnJlYXNvbgAAAAAH0AAAAA1EaXNwdXRlUmVhc29uAAAAAAAAAAAAAAhyZXNvbHZlZAAAAAE=",
        "AAAAAgAAAAAAAAAAAAAAB091dGNvbWUAAAAABAAAAAAAAAAAAAAAEVJlbGVhc2VUb1N1cHBsaWVyAAAAAAAAAQAAAAAAAAAMUmVmdW5kTWVtYmVyAAAAAQAAAAQAAAABAAAAAAAAAAVTcGxpdAAAAAAAAAEAAAAEAAAAAAAAAAAAAAAKUmVmdW5kUG9vbAAA",
        "AAAAAgAAAAAAAAAAAAAADURpc3B1dGVSZWFzb24AAAAAAAAGAAAAAAAAAAAAAAAFU2hvcnQAAAAAAAAAAAAAAAAAAAlXcm9uZ0l0ZW0AAAAAAAAAAAAAAAAAAAdEYW1hZ2VkAAAAAAAAAAAAAAAAB1F1YWxpdHkAAAAAAAAAAAAAAAAMTm90RGVsaXZlcmVkAAAAAAAAAAAAAAAFT3RoZXIAAAA=" ]),
      options
    )
  }
  public readonly fromJSON = {
    open: this.txFromJSON<u64>,
        dispute: this.txFromJSON<Dispute>,
        resolve: this.txFromJSON<null>,
        timeout: this.txFromJSON<null>,
        add_evidence: this.txFromJSON<null>,
        open_disputes_of: this.txFromJSON<Array<u64>>
  }
}