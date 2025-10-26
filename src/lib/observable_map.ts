export class ObservableMap<K, V> {
  private inner: Map<K, V>;
  private set_listeners: Map<K, (value: V) => void>;
  private delete_listeners: Map<K, () => void>;

  constructor() {
    this.inner = new Map();
    this.set_listeners = new Map();
    this.delete_listeners = new Map();
  }
  listen_set(key: K, callback: (value: V) => void) {
    this.set_listeners.set(key, callback);
  }
  listen_delete(key: K, callback: () => void) {
    this.delete_listeners.set(key, callback);
  }
  set(key: K, value: V) {
    this.inner.set(key, value);
    this.set_listeners.get(key)?.(value);
  }
  get(key: K) {
    return this.inner.get(key);
  }
  delete(key: K) {
    this.inner.delete(key);
    this.set_listeners.delete(key);
    this.delete_listeners.get(key)?.();
    this.delete_listeners.delete(key);
  }
}
