export interface Instance {
  free(): void | Promise<void>;
}
