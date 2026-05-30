#!/usr/bin/env bash
# 生成开发用自签证书（仅供本地测试，切勿用于生产）。
#
# 证书的主域名为 relay.xiangqi.example.com，并在 SAN 中额外包含
# localhost / 127.0.0.1 / ::1，方便本机以 insecure 模式联调。
#
# 用法:
#   ./certs/gen-dev-cert.sh
# 产物:
#   certs/relay.crt  (PEM 证书)
#   certs/relay.key  (PEM 私钥, PKCS#8)
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRT="$DIR/relay.crt"
KEY="$DIR/relay.key"
CN="relay.xiangqi.example.com"

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$KEY" -out "$CRT" \
  -days 3650 -sha256 \
  -subj "/CN=$CN" \
  -addext "subjectAltName=DNS:$CN,DNS:localhost,IP:127.0.0.1,IP:::1"

echo "已生成:"
echo "  $CRT"
echo "  $KEY"
echo "主域名: $CN (SAN 含 localhost/127.0.0.1/::1)"
