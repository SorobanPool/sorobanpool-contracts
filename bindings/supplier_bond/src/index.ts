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
  500: {message:"InvalidAmount"},
  501: {message:"InsufficientFreeBond"},
  502: {message:"NotGroupBuy"},
  503: {message:"NotInitialised"},
  504: {message:"ReleaseExceedsReserved"}
}

export interface Client {
  /**
   * Construct and simulate a slash transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Slashes up to `amount` of the bond to the caller (group_buy) so members can be refunded.
   * Returns the amount actually slashed.
   */
  slash: ({caller, supplier, amount}: {caller: string, supplier: string, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<i128>>

  /**
   * Construct and simulate a bond_of transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  bond_of: ({supplier}: {supplier: string}, options?: MethodOptions) => Promise<AssembledTransaction<readonly [i128, i128]>>

  /**
   * Construct and simulate a deposit transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  deposit: ({supplier, amount}: {supplier: string, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a withdraw transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Only the free part of the bond (total minus outstanding advances).
   */
  withdraw: ({supplier, amount}: {supplier: string, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a release_advance transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  release_advance: ({caller, supplier, amount}: {caller: string, supplier: string, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a reserve_for_advance transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  reserve_for_advance: ({caller, supplier, amount}: {caller: string, supplier: string, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

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
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAABQAAAAAAAAANSW52YWxpZEFtb3VudAAAAAAAAfQAAAAAAAAAFEluc3VmZmljaWVudEZyZWVCb25kAAAB9QAAAAAAAAALTm90R3JvdXBCdXkAAAAB9gAAAAAAAAAOTm90SW5pdGlhbGlzZWQAAAAAAfcAAAAAAAAAFlJlbGVhc2VFeGNlZWRzUmVzZXJ2ZWQAAAAAAfg=",
        "AAAAAAAAAH1TbGFzaGVzIHVwIHRvIGBhbW91bnRgIG9mIHRoZSBib25kIHRvIHRoZSBjYWxsZXIgKGdyb3VwX2J1eSkgc28gbWVtYmVycyBjYW4gYmUgcmVmdW5kZWQuClJldHVybnMgdGhlIGFtb3VudCBhY3R1YWxseSBzbGFzaGVkLgAAAAAAAAVzbGFzaAAAAAAAAAMAAAAAAAAABmNhbGxlcgAAAAAAEwAAAAAAAAAIc3VwcGxpZXIAAAATAAAAAAAAAAZhbW91bnQAAAAAAAsAAAABAAAACw==",
        "AAAAAAAAAAAAAAAHYm9uZF9vZgAAAAABAAAAAAAAAAhzdXBwbGllcgAAABMAAAABAAAD7QAAAAIAAAALAAAACw==",
        "AAAAAAAAAAAAAAAHZGVwb3NpdAAAAAACAAAAAAAAAAhzdXBwbGllcgAAABMAAAAAAAAABmFtb3VudAAAAAAACwAAAAA=",
        "AAAAAAAAAEJPbmx5IHRoZSBmcmVlIHBhcnQgb2YgdGhlIGJvbmQgKHRvdGFsIG1pbnVzIG91dHN0YW5kaW5nIGFkdmFuY2VzKS4AAAAAAAh3aXRoZHJhdwAAAAIAAAAAAAAACHN1cHBsaWVyAAAAEwAAAAAAAAAGYW1vdW50AAAAAAALAAAAAA==",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAEAAAAAAAAABmNvbmZpZwAAAAAAEwAAAAA=",
        "AAAAAAAAAAAAAAAPcmVsZWFzZV9hZHZhbmNlAAAAAAMAAAAAAAAABmNhbGxlcgAAAAAAEwAAAAAAAAAIc3VwcGxpZXIAAAATAAAAAAAAAAZhbW91bnQAAAAAAAsAAAAA",
        "AAAAAAAAAAAAAAATcmVzZXJ2ZV9mb3JfYWR2YW5jZQAAAAADAAAAAAAAAAZjYWxsZXIAAAAAABMAAAAAAAAACHN1cHBsaWVyAAAAEwAAAAAAAAAGYW1vdW50AAAAAAALAAAAAA==" ]),
      options
    )
  }
  public readonly fromJSON = {
    slash: this.txFromJSON<i128>,
        bond_of: this.txFromJSON<readonly [i128, i128]>,
        deposit: this.txFromJSON<null>,
        withdraw: this.txFromJSON<null>,
        release_advance: this.txFromJSON<null>,
        reserve_for_advance: this.txFromJSON<null>
  }
}