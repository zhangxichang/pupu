### 构建应用

#### Web应用

```
bun run build:deps
bun install
bun run build
bun run preview
```

#### 原生应用

```
bun run build:deps
bun install
bun tauri icon public/icon.svg
bun tauri build
```
