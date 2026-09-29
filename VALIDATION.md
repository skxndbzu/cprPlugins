# 0.3.1 兼容验证

验证环境：Windows x86_64、Rust 1.97.0；宿主基于官方 `d83d3eb8`（3.17.0），
保留本地 `openai-basispoints-v2` 补丁。宿主合并提交 `60b25098`，低频日志提交 `942ed945`。
插件继续固定原 SDK 提交 `1c6b5a8f1045781d31a301382f1182f1a5657ec8`，业务源码未改写。

## 已验证

- 插件 `cargo fmt -- --check`、严格 Clippy、`cargo test --locked` 和 release 构建通过。
- 宿主 20 项 BPS 回归测试通过：选中账号分组命中/未命中、普通通道回退、工具转换、
  用量唯一性、分片 SSE、首字交付、取消释放、失败与 incomplete 终态。
- 使用目标宿主真实 `ValidatedPackage` 校验摘要、文件清单、Windows 平台和宿主版本，
  使用 `RpcSession` 与 `ProcessSupervisor` 启动安装包中的 exe，不替换成测试插件。
- 原 0.3.0 包在 3.17.0 被版本范围拒绝；以其支持的 3.15.2 进行准入后，
  原包二进制与新运行时的握手、注册和调用通过，证明其进程协议仍可用。
- 0.3.1 包通过 3.15.2、3.17.0 两个版本值的准入与同一新运行时的进程验证。
  每轮验证 JSON/SSE 两种入口下的模型与组匹配、非匹配模型和空组，共六次调用；
  仅调用一次 `host.middleware.next`，组选择字段与请求正文正确传递，响应句柄保留，正常退出。
- 原包与新包的插件 ID、贡献项 ID、权限、配置 schema、secretFields、state 相同。
  原 0.3.0 包未覆盖，其 SHA-256 为
  `d8e961ba1a64f8d896fc1557efc582ee09e8523c937ea7948a755a6090b9ca80`。

复验程序保存在源码工程 `validation/host-compat.rs`，可复制为目标宿主
`backend/crates/gateway-plugin/runtime/examples/bps_compat.rs`，然后执行：

```powershell
cargo +1.97.0 run --manifest-path backend/Cargo.toml -p gateway-plugin-runtime --example bps_compat --locked -- D:/Project/cpr-plugin-oai-basispoints/dist/local.oai-basispoints-0.3.1-x86_64-pc-windows-msvc.tar.gz 3.17.0
```

验证器的第三个参数 `reject` 用于确认旧包的版本拒绝。验证完移除宿主中的临时 example。
3.15.2 的准入检查使用版本值，不代表在另一套旧宿主完整服务上部署验收；原 SDK 的进程测试另行通过。

## 边界与升级顺序

兼容范围为 3.15.2 至 3.17.0，且必须使用保留 BPS v2 补丁的宿主。
先在旧宿主上传并切换到 0.3.1，保留原配置和绑定，再更新宿主。
原 0.3.0 包的 `<3.16.0` 限制不适用于新宿主，不能跳过此步骤。

本验证的宿主回调使用固定响应，不连接真实 BPS，不读取真实账号凭据。
真实模型请求、费用落库、管理端安装/升级、并发轮换和断线结算仍需部署后集成验收。
宿主整体 Windows 测试的结果与缺口单独报告，专项通过不代表所有测试通过。
