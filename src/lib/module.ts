export interface Module {
  init(): void | Promise<void>;
  free(): void | Promise<void>;
}
