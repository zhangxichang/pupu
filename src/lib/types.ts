export interface PK {
  pk: number;
}
export interface ID {
  id: string;
}
export interface Person {
  name: string;
  avatar?: Uint8Array;
  bio?: string;
}
export type DOMPerson = Omit<Person, "avatar"> & { avatar_url?: string };
