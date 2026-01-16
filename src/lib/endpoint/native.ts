import { createTauRPCProxy, type JsonValue } from "~/generated/ipc_bindings";
import type { Person } from "../types";
import type { Endpoint, EndpointModule } from "./interface";

export class EndpointModuleImpl implements EndpointModule {
  init() {}
  async create_endpoint(secret_key: Uint8Array, person: Person) {
    return await EndpointImpl.new(secret_key, person);
  }
  async generate_secret_key() {
    return Uint8Array.from(
      await createTauRPCProxy().endpoint.generate_secret_key(),
    );
  }
  async get_secret_key_id(secret_key: Uint8Array) {
    return await createTauRPCProxy().endpoint.get_secret_key_id(
      Array.from(secret_key),
    );
  }
}

export class EndpointImpl implements Endpoint {
  private id: bigint;

  private constructor(id: bigint) {
    this.id = id;
  }
  static async new(secret_key: Uint8Array, person: Person) {
    return new EndpointImpl(
      await createTauRPCProxy().endpoint.open_endpoint(
        Array.from(secret_key),
        person as unknown as JsonValue,
      ),
    );
  }
  async close() {
    await createTauRPCProxy().endpoint.close_endpoint(this.id);
  }
}
