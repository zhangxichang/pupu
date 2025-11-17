export interface ID {
  id: string;
}
export interface Person {
  name: string;
  avatar?: Uint8Array;
  bio: string;
}
export interface Message {
  sender_id: string;
  text: string;
}
