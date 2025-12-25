import type { Module } from "../module";
import type { Person } from "../types";

export interface EndpointAdapter extends Module {
  open(secret_key: Uint8Array, person: Person): void | Promise<void>;
  shutdown(): Promise<void>;
}
