# CPR Basis Points 插件 0.3

按实际选中账号的分组控制上游协议。版本为 0.3.1，兼容范围为 CPR 3.15.2 至 3.17.0，
且宿主必须保留 `openai-basispoints-v2` 分组协议补丁。官方原版宿主不包含该定制能力。

## 配置

```json
{
  "models": ["gpt-6-astra"],
  "account_group_ids": ["grp_00000000000000000000000000000001"]
}
```

示例组 ID 请替换为 CPR 中真实的账号组 ID，不是组名，也不是账号 ID。
models 与 account_group_ids 都采用明确白名单；任一列表为空时不接管请求。
不用填写 Key、account_id 或令牌。

CPR 先按原有 Key 权限、调度和租约选择账号，再根据当前请求冻结的账号分组快照判断：
选中账号属于任一配置组且模型匹配，使用 Basis Points；其他账号或模型走原接口。
不会强制选中指定组，也不会扩大 Key 的账号权限。一个 Key 可允许多个组，无须拆 Key。
同一账号属于多个组时，命中任一配置组就使用 Basis Points；严格隔离应避免共享同一账号。
账号移组后，新请求使用更新后的快照，在途请求保留原快照。
停用账号表示退出调度，不是切换协议。

## 安装与升级

1. 升级宿主前，在旧 CPR 的插件管理上传
   `dist/local.oai-basispoints-0.3.1-x86_64-pc-windows-msvc.tar.gz`，切换到 0.3.1。
   原 0.3.0 包声明 `<3.16.0`，不能直接随宿主升级到 3.17.0。
2. 确认原有 `models`、`account_group_ids` 与功能绑定保留。插件 ID、能力 ID 和配置字段没有变化。
3. 停止旧 CPR，保持原数据库/Redis 配置和所需隧道；启动从合并后本地主仓构建的宿主。
   `dist/cpr-basispoints-host-0.3.exe` 是旧产物，源码合并不会自动更新它。
4. 功能绑定使用 attempt。若希望完全按插件参数里的实际账号组区分，绑定的账号组范围留空；
   Key 范围也可留空。绑定中已有的 Key/模型条件仍会额外限制生效范围。
5. 启用插件后测试。首次更新宿主需重启，此后插件启停和参数更新沿用 CPR 动态机制。

旧 0.2 配置没有 account_group_ids，升级后默认不接管任何请求。
不要将旧 request 阶段绑定带到新版本。Windows 包不能用于 Linux CPR。

## 调用与确认

POST http://127.0.0.1:8080/v1/responses
请求头 Authorization: Bearer <你的 Key>、Content-Type: application/json

```json
{"model":"gpt-6-astra","input":"请简短回复：测试成功。","stream":false}
```

模型不要加 -basispoints 后缀。核对 CPR 请求记录中的实际账号和 usage/费用；
新宿主在指定组账号成功完成 BPS 响应后，会输出“账号正在使用 BPS，Basis Points 响应已成功完成”，
附账号名称、ID、模型和请求 ID；每个账号首次成功即记录，之后最多每 5 分钟一条。
旧的逐请求选路与 HTTP 状态提示在新宿主中属于 DEBUG 日志。
仅返回“测试成功”不能证明经过插件。要分别验证两组，测试 Key 的权限可临时限定到各组，
不能期待混合组请求必然轮到某个账号。

## 实现与限制

仅声明 requests 权限，调用一次 next，不读取账号凭据、不自建 HTTP 或计费链。
插件向 v2 协议传递组选择条件；宿主读取已经冻结的账号目录，发送前移除内部选择字段。
不查询数据库，不增加分组缓存，不改 SDK、Core、调度或计费。
上游协议沿用 0.2 的 CPA 0.1.18 移植，不改变 Excel 请求头、工具中继或会话标识。

支持文本、完整历史、function/custom 工具、一级 namespace、tool_choice 和 reasoning effort。
文本增量转发，工具等终态校验后交付，usage 经原 canonical decoder。
不支持图片上传、previous_response_id、compact、结构化输出及 priority 档位；
上游 SSE 校验缓冲上限 8 MiB。真实上游和账单落库未验收，见 VALIDATION.md。

## 构建

```powershell
cargo test --locked
cargo build --release --locked
cargo run --manifest-path D:/Project/codex-proxy-rs/backend/Cargo.toml -p codex-proxy-plugin-cli --locked -- package --manifest D:/Project/cpr-plugin-oai-basispoints/plugin.json --binary D:/Project/cpr-plugin-oai-basispoints/target/release/cpr-plugin-oai-basispoints.exe --target x86_64-pc-windows-msvc --output-dir D:/Project/cpr-plugin-oai-basispoints/dist
```
