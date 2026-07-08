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
  200: {message:"AlreadyRegistered"},
  201: {message:"NotRegistered"},
  202: {message:"NotAttestor"},
  203: {message:"NotAdmin"},
  204: {message:"NotAuthorizedCaller"},
  205: {message:"InvalidLevel"},
  206: {message:"BadStatus"}
}

export type Role = {tag: "Trader", values: void} | {tag: "Organizer", values: void} | {tag: "Supplier", values: void};

export type Status = {tag: "Unregistered", values: void} | {tag: "Registered", values: void} | {tag: "Verified", values: void} | {tag: "Suspended", values: void} | {tag: "Closed", values: void};

export interface Client {
  /**
   * Construct and simulate a level transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  level: ({user, role}: {user: string, role: Role}, options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a attest transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Writes a verification hash and level. Level 0 is not a verification.
   */
  attest: ({attestor, user, role, ver_hash, level}: {attestor: string, user: string, role: Role, ver_hash: Buffer, level: u32}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a revoke transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  revoke: ({admin, user, role, reason}: {admin: string, user: string, role: Role, reason: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a status transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  status: ({user, role}: {user: string, role: Role}, options?: MethodOptions) => Promise<AssembledTransaction<Status>>

  /**
   * Construct and simulate a suspend transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Admin, or the `group_buy` / `disputes` contracts.
   */
  suspend: ({caller, user, role, reason}: {caller: string, user: string, role: Role, reason: string}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a register transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  register: ({user, role, profile_hash, cluster}: {user: string, role: Role, profile_hash: Buffer, cluster: Option<string>}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a unsuspend transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  unsuspend: ({admin, user, role}: {admin: string, user: string, role: Role}, options?: MethodOptions) => Promise<AssembledTransaction<null>>

  /**
   * Construct and simulate a cluster_of transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  cluster_of: ({user}: {user: string}, options?: MethodOptions) => Promise<AssembledTransaction<Option<string>>>

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
      new ContractSpec([ "AAAABAAAAAAAAAAAAAAABUVycm9yAAAAAAAABwAAAAAAAAARQWxyZWFkeVJlZ2lzdGVyZWQAAAAAAADIAAAAAAAAAA1Ob3RSZWdpc3RlcmVkAAAAAAAAyQAAAAAAAAALTm90QXR0ZXN0b3IAAAAAygAAAAAAAAAITm90QWRtaW4AAADLAAAAAAAAABNOb3RBdXRob3JpemVkQ2FsbGVyAAAAAMwAAAAAAAAADEludmFsaWRMZXZlbAAAAM0AAAAAAAAACUJhZFN0YXR1cwAAAAAAAM4=",
        "AAAAAAAAAAAAAAAFbGV2ZWwAAAAAAAACAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAEcm9sZQAAB9AAAAAEUm9sZQAAAAEAAAAE",
        "AAAAAAAAAERXcml0ZXMgYSB2ZXJpZmljYXRpb24gaGFzaCBhbmQgbGV2ZWwuIExldmVsIDAgaXMgbm90IGEgdmVyaWZpY2F0aW9uLgAAAAZhdHRlc3QAAAAAAAUAAAAAAAAACGF0dGVzdG9yAAAAEwAAAAAAAAAEdXNlcgAAABMAAAAAAAAABHJvbGUAAAfQAAAABFJvbGUAAAAAAAAACHZlcl9oYXNoAAAD7gAAACAAAAAAAAAABWxldmVsAAAAAAAABAAAAAA=",
        "AAAAAAAAAAAAAAAGcmV2b2tlAAAAAAAEAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAABHVzZXIAAAATAAAAAAAAAARyb2xlAAAH0AAAAARSb2xlAAAAAAAAAAZyZWFzb24AAAAAABEAAAAA",
        "AAAAAAAAAAAAAAAGc3RhdHVzAAAAAAACAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAEcm9sZQAAB9AAAAAEUm9sZQAAAAEAAAfQAAAABlN0YXR1cwAA",
        "AAAAAAAAADFBZG1pbiwgb3IgdGhlIGBncm91cF9idXlgIC8gYGRpc3B1dGVzYCBjb250cmFjdHMuAAAAAAAAB3N1c3BlbmQAAAAABAAAAAAAAAAGY2FsbGVyAAAAAAATAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAEcm9sZQAAB9AAAAAEUm9sZQAAAAAAAAAGcmVhc29uAAAAAAARAAAAAA==",
        "AAAAAAAAAAAAAAAIcmVnaXN0ZXIAAAAEAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAEcm9sZQAAB9AAAAAEUm9sZQAAAAAAAAAMcHJvZmlsZV9oYXNoAAAD7gAAACAAAAAAAAAAB2NsdXN0ZXIAAAAD6AAAABEAAAAA",
        "AAAAAAAAAAAAAAAJdW5zdXNwZW5kAAAAAAAAAwAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAAR1c2VyAAAAEwAAAAAAAAAEcm9sZQAAB9AAAAAEUm9sZQAAAAA=",
        "AAAAAAAAAAAAAAAKY2x1c3Rlcl9vZgAAAAAAAQAAAAAAAAAEdXNlcgAAABMAAAABAAAD6AAAABE=",
        "AAAAAAAAAAAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAEAAAAAAAAABmNvbmZpZwAAAAAAEwAAAAA=",
        "AAAAAgAAAAAAAAAAAAAABFJvbGUAAAADAAAAAAAAAAAAAAAGVHJhZGVyAAAAAAAAAAAAAAAAAAlPcmdhbml6ZXIAAAAAAAAAAAAAAAAAAAhTdXBwbGllcg==",
        "AAAAAgAAAAAAAAAAAAAABlN0YXR1cwAAAAAABQAAAAAAAAAAAAAADFVucmVnaXN0ZXJlZAAAAAAAAAAAAAAAClJlZ2lzdGVyZWQAAAAAAAAAAAAAAAAACFZlcmlmaWVkAAAAAAAAAAAAAAAJU3VzcGVuZGVkAAAAAAAAAAAAAAAAAAAGQ2xvc2VkAAA=" ]),
      options
    )
  }
  public readonly fromJSON = {
    level: this.txFromJSON<u32>,
        attest: this.txFromJSON<null>,
        revoke: this.txFromJSON<null>,
        status: this.txFromJSON<Status>,
        suspend: this.txFromJSON<null>,
        register: this.txFromJSON<null>,
        unsuspend: this.txFromJSON<null>,
        cluster_of: this.txFromJSON<Option<string>>
  }
}