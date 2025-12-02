# Starlink

这是一个使用了Web技术的跨平台网络聊天应用，可以作为Web应用运行，也可以作为一个原生应用运行。

## 主要特性

- **跨平台支持**：可在多种设备和操作系统之间顺畅运行。
- **Web 应用**：无需安装，直接通过浏览器访问[星链](starlink.zhangxichang.com)即可使用。
- **高性能后端**：Web应用Wasm和原生应用后端都由 Rust 编写，兼顾安全与性能。

## 项目使用

本项目是使用了Web技术的跨平台应用，Web和原生应用都可单独开发与运行。

### 安装依赖

这是开始一切前都需要进行的操作

   ```bash
   bun run build:deps
   bun install
   ```

### Web开发与构建

#### 开发

1. 启动开发服务
   ```bash
   bun run dev
   ```
2. [访问开发访问](http://localhost:5173)

#### 构建

1. 构建
   ```bash
   bun run build
   ```
2. 构建输出目录`dist`

### 原生应用开发与构建

#### 安装依赖

原生应用有其自身开始之前需要进行的操作

   ```bash
   bun tauri icon public/icon.svg
   ```

#### 开发

1. 启动开发服务并构建运行应用
   ```bash
   bun tauri dev
   ```

#### 构建

1. 启动开发服务并构建运行应用
   ```bash
   bun tauri build
   ```
   如果要构建用于调试的应用可以使用
   ```bash
   bun tauri build --debug
   ```
2. 构建输出目录`target/release` 调试构建输出目录`target/debug`

## 参与开发

欢迎参与贡献，请参考以下流程：

1. [提交 Issue](https://github.com/ZhangXiChang/starlink/issues)
2. Fork 本仓库
3. 创建您的分支 (`git checkout -b issues/<Issue ID>`)
4. 提交您的更改
5. 推送分支
6. 提交 Pull Request

---

如有任何问题或建议，请[提交 Issue](https://github.com/ZhangXiChang/starlink/issues)。
