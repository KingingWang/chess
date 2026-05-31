# 发布与全平台静态编译说明

本仓库会产出两个独立的发布二进制：

| 二进制 | 路径 | 链接形态 | 备注 |
|---|---|---|---|
| `chess-relay` | 公网中继服务器 | **完全静态**（musl 静态 PIE，无任何 `.so` 依赖） | 一份二进制可直接放到任何 glibc/musl Linux 主机运行 |
| `chess` | Bevy 桌面客户端 | Rust 部分静态，系统图形/音频库动态 | 这是 Bevy/wgpu 的硬约束，下文详述 |

字体（`assets/fonts/cjk.otf` 与 `cjk-bold.otf`）已经通过 `include_bytes!` 直接嵌入 `chess` 二进制，**运行时不再需要 `assets/` 目录**，可以把单个可执行文件分发出去。

---

## 一键构建

```bash
# 构建全部能在当前主机上构建的产物
scripts/build-release.sh

# 只构建中继 / 只构建客户端
scripts/build-release.sh relay
scripts/build-release.sh client

# 想让二进制小一半就 strip 一下
STRIP=1 scripts/build-release.sh
```

产物会被复制到 `dist/`，并打印 `file` 与 `ldd` 输出，方便确认链接形态。

---

## 中继服务器：完全静态

`chess-relay` 用 rustls + ring + argon2 等纯 Rust（或带 ASM 的）依赖，没有 OpenSSL、没有 C 库回退。直接 musl 静态编译：

```bash
rustup target add x86_64-unknown-linux-musl
RUSTFLAGS="-C target-feature=+crt-static" \
    cargo build --release -p chess-relay --target x86_64-unknown-linux-musl
```

产物约 3.7 MB，`file` 显示 `static-pie linked`，`ldd` 显示 `statically linked`。Docker/裸机/容器/Alpine/Debian/CentOS 都能跑。

---

## 桌面客户端：为什么不能"全静态"？

Bevy 0.18 的窗口与渲染栈在 Linux 上依赖：

* **winit / wayland-client / x11-dl** — Wayland/X11 客户端协议库 dlopen 系统 .so
* **wgpu / vulkano / GL** — Vulkan/OpenGL ICD 必须由系统 loader 在运行时寻找显卡驱动
* **alsa-sys / pulse / pipewire** — 音频后端只能用系统库

这些库在所有桌面平台上都是"由发行版/系统管理"的，**不可能 musl 静态进二进制**——否则不同显卡驱动/不同 Wayland 合成器/不同声卡都跑不了。这是 Bevy 官方游戏（Tiny Glade、Foresight 等）以及所有 wgpu 应用的统一发布形态。

我们能做、也已经做了的事：

1. **Rust 标准库与所有 Rust 依赖统一静态链接到单一二进制**（默认 `cargo build --release` 的行为）；
2. **字体资源 `include_bytes!` 内嵌**，运行时不依赖外部目录；
3. `release` profile 已设 `lto = "thin"`、`codegen-units = 1`、`panic = "abort"`；
4. 通过 `--config` 命令行参数或环境变量切换中继配置，**单一二进制 + 单一可选配置文件**即可分发。

各平台对应的"系统动态库"清单：

| 平台 | 必须动态的系统库 | 备注 |
|---|---|---|
| Linux (x86_64) | `libwayland-client.so.0`, `libX11.so.6`, `libudev.so.1`, `libasound.so.2`, `libGL/libEGL/libvulkan` | 大多数桌面发行版默认安装 |
| Windows | `d3d12.dll`, `dxgi.dll`, `xaudio2_9.dll` 等 | 系统自带，无需打包 |
| macOS | `Metal.framework`, `CoreAudio.framework`, `AppKit.framework` | 系统自带，无需打包 |

实测当前 `chess` 在 Linux 上的依赖（任务后 `ldd` 输出）：

```
linux-vdso.so.1
libwayland-client.so.0
libudev.so.1
libasound.so.2
libgcc_s.so.1
libm.so.6
libc.so.6
libffi.so.8
ld-linux-x86-64.so.2
```

如果目标机器缺以上库，提示用户 `apt install libwayland-client0 libxkbcommon0 libasound2 libudev1` 即可。

---

## 跨平台编译

### Windows 客户端（在 Linux 上交叉编译）

```bash
rustup target add x86_64-pc-windows-gnu
# 需要 MinGW: apt install mingw-w64
cargo build --release -p chess-app --target x86_64-pc-windows-gnu
# 产物：target/x86_64-pc-windows-gnu/release/chess.exe
```

### macOS 客户端

Apple 不允许第三方主机交叉链接 `Metal.framework`，**必须在 macOS 上原生编译**：

```bash
cargo build --release -p chess-app                          # 当前 arch
cargo build --release -p chess-app --target aarch64-apple-darwin
cargo build --release -p chess-app --target x86_64-apple-darwin
```

### Windows 服务器（如果想跑在 Windows 上做中继）

```bash
cargo build --release -p chess-relay --target x86_64-pc-windows-gnu
```

---

## 部署中继服务器

1. 把 `dist/chess-relay-linux-x86_64` 上传到服务器；
2. 把 TLS 证书放在 `/etc/chess-relay/{cert.pem,key.pem}`（自签或 Let's Encrypt 都行；客户端默认信任系统根证书或在配置文件中自定义 CA）；
3. 写一份 `/etc/chess-relay/relay.toml`（可选；不写则用编译期默认值，监听 `0.0.0.0:9443`）：
   ```toml
   listen_addr = "0.0.0.0:9443"
   cert_path = "/etc/chess-relay/cert.pem"
   key_path = "/etc/chess-relay/key.pem"
   ```
4. systemd 单元（示例）：
   ```ini
   [Unit]
   Description=Xiangqi relay
   After=network.target

   [Service]
   ExecStart=/usr/local/bin/chess-relay --config /etc/chess-relay/relay.toml
   Restart=always
   User=nobody

   [Install]
   WantedBy=multi-user.target
   ```

中继服务器对游戏数据**零知识**：所有联机数据已经在两个客户端之间通过 Argon2id(房号+密码) 派生的密钥做 AEAD 加密，中继只看到加密块和路由元数据。
