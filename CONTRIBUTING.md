# 贡献指南 (Contributing)

感谢你对 Axolotl Launcher 及其相关内容感兴趣！在提交代码前，请先阅读以下指南。

我们希望所有参与者都能在友善、包容的环境中协作。请先阅读[行为准则 (CODE_OF_CONDUCT.md)](CODE_OF_CONDUCT.md)，了解社区期望和问题报告方式。

## 本地开发

### 环境要求

- **Node.js**：以 [`.nvmrc`](.nvmrc) 为准
- **pnpm**：以根目录 [`package.json`](package.json) 的 `packageManager` 为准
- **Rust**：以 [`rust-toolchain.toml`](rust-toolchain.toml) 为准
- [Tauri v2 系统依赖](https://v2.tauri.app/start/prerequisites/)

### 启动开发环境

#### 非 Nix 环境

非 Nix 环境以 [`.nvmrc`](.nvmrc) 指定 Node.js 版本，以根目录 [`package.json`](package.json) 的 `packageManager` 指定 pnpm 版本。不要混用 Nix 提供的 Node.js 或 pnpm。

1. 初始化 Git 子模块（cubiomes 是构建期的编译依赖，缺失会导致 Rust 编译失败）：
   ```bash
   git submodule update --init --recursive
   ```
2. 使用 `.nvmrc` 安装并激活 Node.js。仅当 Node.js 安装目录可写且不由 Nix 管理时，启用 Corepack：
   ```bash
   corepack enable
   ```
   如果 Node.js 安装目录不可写或由系统包管理器管理，请确保 `$HOME/.local/bin` 在 `PATH` 中，并将 Corepack shim 明确安装到该目录：
   ```bash
   export PATH="$HOME/.local/bin:$PATH"
   corepack enable --install-directory "$HOME/.local/bin"
   ```
   Corepack 的缓存环境变量不会改变 shim 的安装目录。
3. 在仓库根目录安装锁定的依赖，然后启动开发服务器：
   ```bash
   pnpm install --frozen-lockfile
   pnpm app:dev
   ```

#### Nix 开发环境

在受支持的 `x86_64-linux` 或 `aarch64-linux` 系统上，可使用仓库已提交的 `flake.lock` 进入固定版本的开发环境。该环境不会自动初始化子模块或安装 JavaScript 依赖：

```bash
git submodule update --init --recursive
nix develop
pnpm install --frozen-lockfile
pnpm app:dev
```

进入 `nix develop` 后不要运行 `corepack enable`；flake 已将独立固定版本的 pnpm 10.33.2 直接放入 `PATH`。进入开发环境后，可直接运行下方现有检查命令。

#### direnv 自动加载

已安装 [direnv](https://direnv.net/) 并为当前 Shell 配置 hook 后，仓库根目录的 `.envrc` 会通过 `use flake` 自动加载同一个默认 Nix 开发环境。首次进入仓库时执行：

```bash
git submodule update --init --recursive
direnv allow
pnpm install --frozen-lockfile
pnpm app:dev
```

direnv 环境中也不要运行 `corepack enable`；flake 已提供独立固定版本的 pnpm 10.33.2。`.envrc` 不会初始化子模块或安装依赖；direnv 的本地缓存目录 `.direnv/` 已被 Git 忽略。`.envrc` 内容变更后，需要重新运行 `direnv allow`。

> `nix build .` 使用 `fetchPnpmDeps` 和 `pnpmConfigHook` 完成封闭构建，`nix run .` 直接启动已打包的原生二进制文件且不需要 Node.js 或 pnpm；二者成功不代表普通宿主 Shell 的开发工具链配置正确。

### 常用检查命令

在提交代码前，建议运行以下命令确保代码符合规范：

```powershell
# 品牌名和文本合规检查
pnpm axolotl:brand-guard
# 国际化词条检查
pnpm axolotl:i18n-check
# 前端格式化及 lint
pnpm prepr:frontend:app
# Rust 格式化检查
cargo fmt --all --check
# Rust 基础检查
cargo check --package theseus_gui --features updater
```

### 构建缓存与磁盘空间

Rust 编译产物位于 `target` 目录，首次完整构建可能占用数 GB 空间。Turbo 仅缓存前端输出，不会缓存 `target/**`。桌面应用的 Tauri 构建任务已明确关闭 Turbo 缓存。

如需释放本地开发缓存，可以删除以下目录：

```powershell
Remove-Item -Recurse -Force .turbo\cache
Remove-Item -Recurse -Force target\debug
```

此操作不会删除 `target\installer-test` 中单独生成的安装包。下次启动开发模式时需要重新编译 Rust 依赖。

## 仓库范围

Axolotl 的产品改动主要位于：

- `apps/app-frontend`
- `apps/app`
- `packages/app-lib`
- 上述包所需的共享 UI 与资源包

本仓库**不包含** Modrinth 网站、Labrinth API 或其运营服务源码。桌面端保留对 Modrinth 公共 API 的客户端兼容；如果需要参考上游实现，请仅手动挑选与 Axolotl 产品相关的改动，避免直接合并无关代码。

## 发布新版本

发布流程由 [`.github/workflows/axolotl-release.yml`](.github/workflows/axolotl-release.yml) 自动完成。版本号以 Git 标签为准，必须符合[语义化版本格式](https://semver.org/lang/zh-CN/)。

打标签并推送到远端即可触发发布工作流：

```powershell
git tag -a v1.2.3 -m "Axolotl Launcher 1.2.3"
git push origin v1.2.3
```

预发布版本请使用带后缀的标签（如 `v1.2.3-beta.1`）。

**自动发布工作流执行步骤**：

1. 将标签版本写入桌面应用构建配置。
2. 在 GitHub 托管的 Windows、macOS 和 Linux runner 上并行构建安装包。
3. 使用仓库 Secrets 中的 Tauri 私钥生成签名更新包。
4. 生成并校验包含全部桌面平台的 `latest.json`。
5. 校验成功后将草稿 Release 转为正式发布。
6. GitHub Release 发布完成后，将安装包、更新清单和 Release 信息镜像到 CNB。

源码分支和标签也由 GitHub Actions 在 push 和删除事件后直接同步到 CNB。CNB 仓库不运行定时同步或标签流水线，避免在 GitHub 构建尚未完成时占用 CNB 构建时长。

> 注意：自动更新公钥已固化在客户端中，私钥只保存在 GitHub Actions Secrets 中，切勿提交到仓库。
