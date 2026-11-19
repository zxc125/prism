---
name: deploy
description: Console 私有化部署与升级流程（observer-server → 192.168.17.108）。当用户提到部署、上线、升级 server、更新服务器上的 console、prism-deploy.tar.gz、install.sh、scp 传包、192.168.17.108、observer-server 二进制构建、WSL musl 编译时使用本 skill。涵盖一键构建脚本、服务器端安装脚本、WSL 一次性环境依赖、历史踩坑与排障表。
---

# Console 私有化部署（192.168.17.108）

把本项目的 console（observer-server + 前端 dist）部署/升级到私有服务器。**脚本化的动作直接跑脚本，不要重新发明流程**；本 skill 提供流程上下文、环境依赖清单和踩坑记录。

## 服务器现状（2026-11-19 部署）

| 项 | 值 |
| --- | --- |
| 地址 | `192.168.17.108`（zhijian@zhijian-virtual-machine，LAN 虚拟机） |
| 二进制 | `/usr/local/bin/observer-server` |
| 静态文件 | `/opt/prism/web` |
| 数据目录 | `/var/lib/prism/recordings`（**升级永不碰**，向前兼容） |
| 端口 / token | `0.0.0.0:8080` / `sk_987654321` |
| 运行方式 | root；install.sh 自动探测 systemd 单元或裸进程，按原方式拉起 |

## 标准更新流程（跑通版）

```sh
# 1. 本机：一键出包（pnpm build + WSL musl 增量编译 → prism-deploy.tar.gz，约 1 分钟）
bash scripts/deploy-console.sh

# 2. 本机：传包
scp prism-deploy.tar.gz zhijian@192.168.17.108:/tmp/

# 3. 服务器：解压执行安装（自动停旧→换文件→按原方式重启，数据不碰）
sudo rm -rf /tmp/prism-deploy && mkdir /tmp/prism-deploy
sudo tar -xzf /tmp/prism-deploy.tar.gz -C /tmp/prism-deploy
sudo /tmp/prism-deploy/install.sh

# 4. 验证
curl -s http://127.0.0.1:8080/whoami -H "Authorization: Bearer sk_987654321"
# 浏览器: http://192.168.17.108:8080
```

两个脚本是流程的唯一事实源：
- [scripts/deploy-console.sh](../../../scripts/deploy-console.sh) — 本机构建 + 打包（tar 内含 install.sh）
- [scripts/install.sh](../../../scripts/install.sh) — 服务器端安装（头注释含服务器布局）

## 首次部署（新机器/新服务器）

服务器不需要 git/node/cargo——全部产物在本机构建好后以 tarball 交付。步骤：改 `scripts/install.sh` 头部的 BIN/WEB/DATA/BIND/TOKEN 常量为新布局 → 跑上面流程 1-3。服务器上装 systemd（推荐，unit 模板见 `site/docs/deploy.md`）或裸 nohup 均可，install.sh 两者都会处理。

## WSL 一次性环境（本机构建依赖，勿重装勿重置）

Windows 本机用 WSL Ubuntu 编 Linux 二进制。以下已装好，**重置 WSL 后需按此清单重装**：

1. **rustup + stable**：官方源 `https://sh.rustup.rs` 直装（WSL 直连官方可行；rsproxy/TUNA/USTC 的 rustup 镜像路径全 403/404，别浪费时间）
2. **musl target 标准库**：rustup 官方源下载龟速（5s/352B），用 **Windows 侧 8 路并行分段下载**（见踩坑 #4），组件装进 gnu 工具链：`./install.sh --prefix=$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu`
3. **build-essential**：WSL 裸系统没有 `cc`，serde 等编译需要；`wsl -u root apt-get install -y build-essential`（root 免密）
4. **公共 DNS**：WSL 默认走 Windows 网关 DNS（172.x），解析不了国内 CDN 域名；`wsl -u root` 写 `/etc/resolv.conf` 为 `223.5.5.5` + `119.29.29.29`
5. **cargo 走 rsproxy-sparse**：`~/.cargo/config.toml` 里 `replace-with = "rsproxy-sparse"` → `sparse+https://rsproxy.cn/index/`

## 踩坑记录（遇到同类问题先查这里）

1. **Windows cargo 编不了 Linux 二进制** —— 二进制必须 Linux 侧编；WSL 是现成 Linux，musl 模式静态链接后不挑服务器 glibc 版本（observer-server 全纯 Rust 依赖，musl 零障碍）。
2. **rustup 镜像 403/404** —— rsproxy.cn 不镜像 rustup dist；TUNA/USTC 对 rustup-init 403。官方源能通（206），只是慢。
3. **dist 路径带日期不带版本** —— 组件包 URL 形如 `static.rust-lang.org/dist/<日期>/rust-std-...tar.gz`；按版本号拼路径必 404。真实 URL 从 `channel-rust-stable.toml` manifest 查。
4. **大文件下载慢 → 8 路并行分段** —— 单连接 ~32KB/s 时，`curl -r start-end` 分 8 段并行（每段约 5MB）再 `cat` 拼回，校验总字节数。41MB 从 20 分钟降到 2 分钟。
5. **WSL DNS 解析失败** —— `Could not resolve host: lf6-static.rsproxy.cn` 这类 = WSL 网关 DNS 不行的信号，换公共 DNS（见环境清单 #4）。
6. **cp -r 拷 crate 进已有构建目录会嵌套** —— `cp -r src ~/prism-build/` 在目标已存在时把目录拷进去变 `observer-server/observer-server`，且 `rm -rf` 整目录会丢 target 缓存。deploy-console.sh 已按「只拷 src + Cargo.toml、保 target」写好，别改回整目录拷贝。
7. **Windows 写的 shell 脚本进 Linux 要去 CRLF** —— install.sh 打包前 `sed -i "s/\r$//"`（deploy-console.sh 已含）。
8. **observer-server 的 --web-dir 不存在时静默跳过静态托管** —— API 正常但浏览器白屏；先查路径。

## 排障表

| 现象 | 多半是 | 解法 |
| --- | --- | --- |
| WSL 编译报 `linker cc not found` | build-essential 没装 | 环境清单 #3 |
| cargo 下载 `Could not resolve host` | WSL DNS | 环境清单 #4 |
| 编译中途卡死无输出 | 官方源龟速下载依赖 | 确认 cargo 用 rsproxy-sparse（环境清单 #5） |
| scp 后 install.sh 报 `\r` 相关错 | CRLF | 脚本已自动处理；手跑时先 `sed -i "s/\r$//"` |
| 升级后浏览器白屏、API 正常 | web-dir 路径不对 | 查 `/opt/prism/web` 是否为新 dist（踩坑 #8） |
| /whoami 401 | token 不对 | 对照 install.sh 的 TOKEN 常量 |
| 旧进程被 kill 又自己活了 | systemd Restart=on-failure 在管 | install.sh 会检测 unit 并 stop/start，别手动 pkill |

## 边界

- 只覆盖 **observer-server 自托管** 这条部署线（192.168.17.108 及同构服务器）。Tauri 桌面版打包（`pnpm tauri build`）和官网/文档站（`pnpm build:site` + GH Pages）不在此列。
- 通用部署手册（面向外部用户）在 [site/docs/deploy.md](../../../site/docs/deploy.md)；本 skill 是**本项目自有服务器的运维流程**。
