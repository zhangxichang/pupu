export type PersonProtocolEvent = "FriendRequest" | "ChatRequest";

export interface Person {
  name: string;
  avatar?: Uint8Array;
  bio: string;
}

export interface RelayConfig {
  url: string;
  quic_port: number;
}
