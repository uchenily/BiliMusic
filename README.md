<div align="center">
  <img src="./src/assets/icon.png" width="112" height="112" alt="BiliMusic Logo" />
  <h1>BiliMusic</h1>
  <p><strong>把 Bilibili 变成一座精致、灵动、像 Apple Music 一样顺手的桌面音乐资料库。</strong></p>
  <p>
    <a href="./README.en.md">English</a> ·
    <a href="https://github.com/HanversionOvO/BiliMusic">GitHub</a>
  </p>
  <p>
    <img alt="React" src="https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=111" />
    <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=for-the-badge&logo=tauri&logoColor=fff" />
    <img alt="Vite" src="https://img.shields.io/badge/Vite-6-646CFF?style=for-the-badge&logo=vite&logoColor=fff" />
    <img alt="Rust" src="https://img.shields.io/badge/Rust-1.80+-000?style=for-the-badge&logo=rust&logoColor=fff" />
  </p>
</div>

## 项目简介

BiliMusic 是一个基于 Bilibili 的桌面音乐播放器，提供搜索、推荐、排行榜、UP 主浏览、播放队列、歌词、收藏、歌单、扫码登录和 WebDAV 同步等功能。

桌面端现已使用 **Tauri 2 + Rust** 实现，不再依赖 Electron，也不再提供 HarmonyOS/OpenHarmony 专用工程。React 负责界面和播放体验，Rust 负责跨域网络请求、Bilibili Cookie、歌词代理、音频下载、WebDAV 和系统托盘。

## 功能

- Bilibili 视频、用户、推荐、热门和音乐榜单
- 分 P 音频播放与正确的 CID 匹配
- Bilibili CC/AI 字幕与 QQ Music 歌词匹配
- 播放队列、随机播放、循环播放、最近播放和收藏
- 自定义歌单以及 JSON 导入/导出
- Bilibili 扫码登录、登录状态持久化和退出登录
- WebDAV 双向同步与 ETag 冲突处理
- Tauri 原生窗口控制、系统托盘和全屏
- Windows、macOS、Linux 桌面安装包

## 技术架构

```text
┌──────────────────────────────────────────────┐
│ React 19 + Vite + TypeScript                 │
│ 页面、播放器、歌词、歌单、前端状态           │
└──────────────────┬───────────────────────────┘
                   │ @tauri-apps/api invoke/event
┌──────────────────▼───────────────────────────┐
│ Tauri 2 Rust backend                         │
│ Bilibili / Lyrics / Download / WebDAV / Tray │
│ Cookie persistence / Window lifecycle        │
└──────────────────────────────────────────────┘
```

网络相关业务通过 `src/services/tauriApi.ts` 调用 Tauri command。Bilibili 的 WBI 签名、数据转换、歌词匹配和播放器业务仍由前端服务层负责。

## 快速开始

### 环境要求

- Node.js 20+
- npm
- Rust stable 与 Cargo
- Windows 开发需要 WebView2；macOS/Linux 需要 Tauri 对应系统依赖

### 安装依赖

```bash
npm install
```

### 浏览器开发模式

```bash
npm run dev
```

### Tauri 桌面开发模式

```bash
npm run tauri:dev
```

### 构建前端

```bash
npm run build
```

### 构建桌面安装包

```bash
npm run tauri:build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 常用脚本

| 命令 | 说明 |
| --- | --- |
| `npm run dev` | 启动 Vite 开发服务器 |
| `npm run build` | 构建 React 前端 |
| `npm run preview` | 预览前端构建产物 |
| `npm run tauri:dev` | 启动 Tauri 开发应用 |
| `npm run tauri:build` | 构建 Tauri 桌面安装包 |

## 项目结构

```text
BiliMusic
├─ src/
│  ├─ components/       播放器、队列、歌词、布局和通用组件
│  ├─ contexts/         播放、登录、播放页和歌单状态
│  ├─ hooks/            主题、设置、歌词和自动同步 hooks
│  ├─ pages/            发现、搜索、推荐、歌单、下载和设置
│  ├─ services/         Bilibili 数据、Tauri bridge、歌词匹配
│  ├─ styles/           全局样式和 Apple Music 风格系统
│  └─ utils/            本地存储、歌单和 WebDAV 同步
├─ src-tauri/
│  ├─ src/lib.rs        Tauri commands、网络代理和托盘
│  ├─ src/main.rs       桌面入口
│  ├─ capabilities/     Tauri 权限配置
│  ├─ icons/             桌面应用图标
│  └─ tauri.conf.json   Tauri 构建和窗口配置
├─ .github/workflows/   多平台构建流程
├─ vite.config.ts
└─ package.json
```

## 数据与隐私

前端播放队列、歌单、收藏、主题和歌词缓存保存在 WebView 的 `localStorage`。Tauri 后端会在应用数据目录保存 Bilibili Cookie、WebDAV 配置和下载文件。

迁移到 Tauri 后，Electron Session 和旧 Electron WebDAV 加密数据不会自动复用，首次使用桌面版可能需要重新扫码登录并重新输入 WebDAV 密码。

## 开发检查

提交前建议运行：

```bash
npm run build
npx tsc --noEmit
cargo check --manifest-path src-tauri/Cargo.toml
```

## 免责声明

BiliMusic 仅用于学习、研究和个人使用。项目与 Bilibili、Apple Music、Apple Inc. 及相关服务提供方没有从属、授权或商业合作关系。请遵守相关平台服务条款、版权规定和当地法律法规。
