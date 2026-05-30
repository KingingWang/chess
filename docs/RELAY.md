# 公网联机中继服务器 (chess-relay)

本项目的联机对弈支持两种方式（在“创建/加入”对话框内通过 **局域网 / 服务器** 切换）：

1. **局域网**：客户端之间直接 TCP 连接（原有逻辑）。
2. **服务器中继**：两端通过公网 `chess-relay` 服务器转发数据。服务器
   **零知识**——只转发不透明的密文字节，看不到任何棋局内容。

## 安全模型（端到端加密）

- 创建/加入房间需要 **房间号 + 密码**。
- 房间号为 **8 位数字**，随机且在“当前活跃房间”内查重，房间关闭后号码回收。
- 密码经 **Argon2id(密码, salt)** 派生出端到端对称密钥。salt 随机生成，
  本身非秘密，经服务器中转给对端；密码本身**绝不**发送给服务器。
- 所有棋局数据在客户端加密、在对端解密。服务器只负责把一端的密文转发到
  另一端。密码是否正确由“能否成功解密握手帧”隐式判定——错误密码会导致
  握手失败。
- 服务器不存储任何密码，也无法还原密钥。

## 传输协议

- 使用 **WSS（WebSocket over TLS）**，与 HTTPS 共用同一张 TLS 证书。
- TLS 栈基于 **rustls + ring**，不依赖 OpenSSL/C，便于全静态编译。

## 配置（服务器与客户端均适用）

优先级（高 → 低）：**配置文件 > 环境变量 > 编译内置默认**。
可用 `--config <路径>` 指定配置文件；不指定时读取运行目录下的默认文件
（服务器 `relay.toml`，客户端 `client.toml`），文件不存在则回退到环境
变量/内置默认。若用 `--config` 明确指定但文件读不到，服务器会报错退出。

### 服务器 (`relay.toml` / 环境变量)

| 配置文件键 | 环境变量 | 默认值 | 说明 |
|---|---|---|---|
| `host` | `CHESS_RELAY_HOST` | `0.0.0.0` | 监听地址 |
| `port` | `CHESS_RELAY_PORT` | `9443` | 监听端口 |
| `cert` | `CHESS_RELAY_CERT` | `certs/relay.crt` | TLS 证书 (PEM) |
| `key`  | `CHESS_RELAY_KEY`  | `certs/relay.key` | TLS 私钥 (PEM) |

模板见 `relay.toml.example`。

### 客户端 (`client.toml` / 环境变量)

| 配置文件键 | 环境变量 | 默认值 | 说明 |
|---|---|---|---|
| `host` | `CHESS_RELAY_HOST` | `relay.xiangqi.example.com` | 服务器域名/IP |
| `port` | `CHESS_RELAY_PORT` | `9443` | 服务器端口 |
| `ca_path` | `CHESS_RELAY_CA` | （无） | 自签/私有 CA 证书路径 |
| `insecure` | `CHESS_RELAY_INSECURE` | `false` | 跳过证书校验（仅调试） |

模板见 `client.toml.example`。

## 开发环境快速开始

1. 生成自签证书（主域名 `relay.xiangqi.example.com`，SAN 含 localhost/127.0.0.1）：

   ```bash
   ./certs/gen-dev-cert.sh
   ```

2. 启动服务器（默认监听 `0.0.0.0:9443`）：

   ```bash
   cargo run -p chess-relay
   # 或指定配置文件
   cargo run -p chess-relay -- --config relay.toml
   ```

3. 客户端本机联调连自签服务器，两种方式任选其一：
   - **信任自签证书**（推荐）：`client.toml` 设 `host = "127.0.0.1"` 并
     `ca_path = "certs/relay.crt"`。
   - **跳过校验**（仅本地）：设 `host = "127.0.0.1"`、`insecure = true`。

   然后在客户端“创建/加入”对话框里切换到 **服务器**：
   - 一方“创建”→ 得到 8 位房间号，连同密码告诉对方；
   - 另一方“加入”→ 输入房间号与相同密码即可开始对弈。

## 全平台静态编译

`chess-relay` 不依赖图形栈，可做到**完全静态**（musl + rustls/ring，无 C 依赖）。
> 注意：`chess-app`（Bevy 图形客户端）因图形/系统库无法完全静态，这是预期内的，两者分开构建。

```bash
# Linux x86_64 (完全静态)
rustup target add x86_64-unknown-linux-musl
cargo build --release -p chess-relay --target x86_64-unknown-linux-musl

# Linux aarch64 (完全静态)
rustup target add aarch64-unknown-linux-musl
cargo build --release -p chess-relay --target aarch64-unknown-linux-musl

# Windows x86_64
rustup target add x86_64-pc-windows-gnu
cargo build --release -p chess-relay --target x86_64-pc-windows-gnu

# macOS (在 macOS 上交叉/本地构建)
cargo build --release -p chess-relay --target aarch64-apple-darwin
```

产物为单文件二进制：`target/<target>/release/chess-relay`。
验证 Linux musl 是否静态：`ldd target/x86_64-unknown-linux-musl/release/chess-relay`
应输出 “not a dynamic executable”。

## 生产部署（Let's Encrypt）

1. 准备一个真实域名（替换内置默认的 `relay.xiangqi.example.com`），DNS 指向服务器。
2. 用 certbot 申请证书：

   ```bash
   sudo certbot certonly --standalone -d your.domain.com
   # 证书位于 /etc/letsencrypt/live/your.domain.com/{fullchain.pem,privkey.pem}
   ```

3. 在 `relay.toml` 指向证书：

   ```toml
   host = "0.0.0.0"
   port = 9443
   cert = "/etc/letsencrypt/live/your.domain.com/fullchain.pem"
   key  = "/etc/letsencrypt/live/your.domain.com/privkey.pem"
   ```

4. 客户端 `client.toml` 设 `host = "your.domain.com"`，由于是受信任 CA 签发，
   无需 `ca_path`，`insecure` 保持 `false`。
5. 证书续期后重启服务即可（或配置 certbot deploy hook 重启服务）。
