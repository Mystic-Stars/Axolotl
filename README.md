# Axolotl Launcher

[![Desktop CI](https://github.com/Mystic-Stars/Axolotl/actions/workflows/axolotl-ci.yml/badge.svg)](https://github.com/Mystic-Stars/Axolotl/actions/workflows/axolotl-ci.yml)
[![Release](https://github.com/Mystic-Stars/Axolotl/actions/workflows/axolotl-release.yml/badge.svg)](https://github.com/Mystic-Stars/Axolotl/actions/workflows/axolotl-release.yml)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](COPYING.md)

Axolotl Launcher 是由 Mystic Stars 开发的免费跨平台 Minecraft 启动器。

本项目基于 Modrinth 单体仓库进行下游开发，专注于桌面启动器体验。它是使用 Modrinth 公共 API 的独立非官方客户端，与 Rinth, Inc. 不存在隶属或背书关系。

> 推荐 API 服务（广告）：[FutureAPI](https://www.futureapi.cc/register?invite=8xmfivnh)

## 功能

- 支持 Windows、macOS 和 Linux
- 管理 Minecraft 实例、整合包、模组、资源包和光影
- 支持 Microsoft 正版账户与离线账户
- 支持实例截图管理、离线皮肤和服务器管理
- 可自定义主题强调色与启动器背景
- 完整的简体中文界面，并保留多语言支持
- 通过签名的 GitHub Release 更新包自动检查和安装新版本

## 下载与安装

前往 [GitHub Releases](https://github.com/Mystic-Stars/Axolotl/releases/latest) 下载适合当前系统的安装包：

| 系统              | 安装包                                |
| ----------------- | ------------------------------------- |
| Windows 10/11 x64 | NSIS `.exe`                           |
| macOS             | 通用 `.dmg`（Intel 与 Apple Silicon） |
| Linux x64         | `.AppImage`、`.deb` 或 `.rpm`         |

每次正式发布都会同时上传 Tauri 签名的更新包和 `latest.json`。已安装的正式版会从当前仓库的最新 Release 检查更新，并在验证签名后安装。

Arch Linux 用户可从 AUR 安装：

```bash
# 源码构建版
yay -S axolotl-launcher

# 预编译二进制版
yay -S axolotl-launcher-bin
```

Debian(及其分支)amd64/arm64 用户可选（APT 安装/更新）：

```bash
curl -fsSL https://ppa.axlmc.org/setup.sh | sudo bash
sudo apt install axolotl-launcher
```

## 本地开发

### 环境要求

- Node.js：以 [`.nvmrc`](.nvmrc) 为准
- pnpm：以根目录 [`package.json`](package.json) 的 `packageManager` 为准
- Rust：以 [`rust-toolchain.toml`](rust-toolchain.toml) 为准
- [Tauri v2 系统依赖](https://v2.tauri.app/start/prerequisites/)

### 启动开发环境

```powershell
corepack enable
pnpm install --frozen-lockfile
pnpm app:dev
```

### 常用检查

```powershell
pnpm axolotl:brand-guard
pnpm axolotl:i18n-check
pnpm prepr:frontend:app
cargo fmt --all --check
cargo check --package theseus_gui --features updater
```

### 构建缓存与磁盘空间

Rust 编译产物位于 `target`，首次完整构建可能占用数 GB。Turbo 只缓存前端输出，不会缓存 `target/**`；桌面应用的 Tauri 构建任务也已明确关闭 Turbo 缓存。需要释放本地开发缓存时，可以仅删除：

```powershell
Remove-Item -Recurse -Force .turbo\cache
Remove-Item -Recurse -Force target\debug
```

这不会删除 `target\installer-test` 中单独生成的安装包，但下次启动开发模式时需要重新编译 Rust 依赖。

## 发布新版本

发布由 [`.github/workflows/axolotl-release.yml`](.github/workflows/axolotl-release.yml) 自动完成。版本号以 Git 标签为准，必须符合语义化版本格式：

```powershell
git tag -a v1.2.3 -m "Axolotl Launcher 1.2.3"
git push origin v1.2.3
```

工作流会依次完成：

1. 将标签版本写入桌面应用构建配置；
2. 在 GitHub 托管的 Windows、macOS 和 Linux runner 上构建安装包；
3. 使用仓库 Secrets 中的 Tauri 私钥生成签名更新包；
4. 生成并校验包含全部桌面平台的 `latest.json`；
5. 校验成功后将草稿 Release 正式发布。

预发布版本使用带后缀的标签，例如 `v1.2.3-beta.1`。自动更新公钥已固化在客户端中，私钥只保存在 GitHub Actions Secrets 中，不应提交到仓库。

## 仓库范围与上游同步

Axolotl 的产品改动主要位于：

- `apps/app-frontend`
- `apps/app`
- `packages/app-lib`
- 上述包所需的共享 UI 与资源包

Modrinth 网站和后端并不是 Axolotl 产品。`upstream` 远程指向 Modrinth 原仓库；上游更新应先审查影响再合并，不应使用强制推送覆盖 Axolotl 的提交历史。

## 隐私与第三方服务

启动器会按用户操作访问 Microsoft/Minecraft 登录服务、Modrinth API、Minecraft 内容服务以及所安装内容声明的第三方下载地址。Axolotl 已禁用原上游的私有服务能力；具体网络端点可在项目配置与源码中审查。

## 许可证

桌面相关包继续使用 **GPL-3.0-only** 许可证。详情请查看各包中的 `LICENSE`、`COPYING.md` 以及仓库根目录的 [`COPYING.md`](COPYING.md)。

官方网站：[https://www.axlmc.org](https://www.axlmc.org)
