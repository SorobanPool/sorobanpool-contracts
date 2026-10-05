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
  600: {message:"NotAuthorizedCaller"},
  601: {message:"UnknownEvent"},
  602: {message:"NotInitialised"}
}

export type Role = {tag: "Trader", values: void} | {tag: "Organizer", values: void} | {tag: "Supplier", values: void};


export interface RepStats {
  disputes_lost: u32;
  disputes_opened: u32;
  disputes_won: u32;
  first_active_at: u64;
  late: u32;
  on_time: u32;
  pickups_confirmed: u32;
  pools_failed: u32;
  pools_settled: u32;
  short_deliveries: u32;
}

export interface Client {
  /**
   * Construct and simulate a tier transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  tier: ({subject, role}: {subject: string, role: Role}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a stats transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  stats: ({subject, role}: {subject: string, role: Role}, options?: MethodOptions) => Promise<AssembledTransaction<RepStats>>

  /**
   * Construct and simulate a record transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Events: settled, failed, on_time, late, d_open, d_won, d_lost, short, pickup.
   * Caller must be the group_buy or disputes contract.
   */
  record: ({caller, subject, role, event}: {caller: string, subject: string, role: Role, event: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

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
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAAAwAAAAAAAAATTm90QXV0aG9yaXplZENhbGxlcgAAAAJYAAAAAAAAAAxVbmtub3duRXZlbnQAAAJZAAAAAAAAAA5Ob3RJbml0aWFsaXNlZAAAAAACWg==",
        "AAAAAAAAAAAAAAAEdGllcgAAAAIAAAAAAAAAB3N1YmplY3QAAAAAEwAAAAAAAAAEcm9sZQAAB9AAAAAEUm9sZQAAAAEAAAAE",
        "AAAAAAAAAAAAAAAFc3RhdHMAAAAAAAACAAAAAAAAAAdzdWJqZWN0AAAAABMAAAAAAAAABHJvbGUAAAfQAAAABFJvbGUAAAABAAAH0AAAAAhSZXBTdGF0cw==",
        "AAAAAAAAAIBFdmVudHM6IHNldHRsZWQsIGZhaWxlZCwgb25fdGltZSwgbGF0ZSwgZF9vcGVuLCBkX3dvbiwgZF9sb3N0LCBzaG9ydCwgcGlja3VwLgpDYWxsZXIgbXVzdCBiZSB0aGUgZ3JvdXBfYnV5IG9yIGRpc3B1dGVzIGNvbnRyYWN0LgAAAAZyZWNvcmQAAAAAAAQAAAAAAAAABmNhbGxlcgAAAAAAEwAAAAAAAAAHc3ViamVjdAAAAAATAAAAAAAAAARyb2xlAAAH0AAAAARSb2xlAAAAAAAAAAVldmVudAAAAAAAABEAAAAA",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAEAAAAAAAAABmNvbmZpZwAAAAAAEwAAAAA=",
        "AAAAAgAAAAAAAAAAAAAABFJvbGUAAAADAAAAAAAAAAAAAAAGVHJhZGVyAAAAAAAAAAAAAAAAAAlPcmdhbml6ZXIAAAAAAAAAAAAAAAAAAAhTdXBwbGllcg==",
        "AAAAAQAAAAAAAAAAAAAACFJlcFN0YXRzAAAACgAAAAAAAAANZGlzcHV0ZXNfbG9zdAAAAAAAAAQAAAAAAAAAD2Rpc3B1dGVzX29wZW5lZAAAAAAEAAAAAAAAAAxkaXNwdXRlc193b24AAAAEAAAAAAAAAA9maXJzdF9hY3RpdmVfYXQAAAAABgAAAAAAAAAEbGF0ZQAAAAQAAAAAAAAAB29uX3RpbWUAAAAABAAAAAAAAAARcGlja3Vwc19jb25maXJtZWQAAAAAAAAEAAAAAAAAAAxwb29sc19mYWlsZWQAAAAEAAAAAAAAAA1wb29sc19zZXR0bGVkAAAAAAAABAAAAAAAAAAQc2hvcnRfZGVsaXZlcmllcwAAAAQ=" ]),
      options
    )
  }
  public readonly fromJSON = {
    tier: this.txFromJSON<u32>,
        stats: this.txFromJSON<RepStats>,
        record: this.txFromJSON<null>
  }
}