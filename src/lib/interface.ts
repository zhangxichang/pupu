export interface Init {
  init(): Promise<void>;
}
export interface Free {
  free(): void | Promise<void>;
}
