# 来源与协议依据

Basis Points 交互参考 JaxsonWang/cpa-plugin-oai-basispoints 0.1.18，固定提交
11df6f8855847ec1957b0d2f4271a9cd9b13cfe1，MIT License，版权声明见 LICENSE。

- [请求协议](https://github.com/JaxsonWang/cpa-plugin-oai-basispoints/blob/11df6f8855847ec1957b0d2f4271a9cd9b13cfe1/internal/basispoints/protocol.go)
- [身份及上游传输](https://github.com/JaxsonWang/cpa-plugin-oai-basispoints/blob/11df6f8855847ec1957b0d2f4271a9cd9b13cfe1/internal/basispoints/upstream.go)

这是第三方观察到的私有客户端协议，并非 OpenAI 承诺稳定的公开接口。
本地宿主 providers/openai/src/transport/basispoints 包含 Rust 协议移植及 MIT 许可副本。
插件本体只使用公开 attempt middleware 选择该协议。

gateway-plugin-sdk 使用 Apache-2.0，固定 CPR 提交 1c6b5a8f1045781d31a301382f1182f1a5657ec8；
许可副本见 licenses/Apache-2.0.txt。CPR 宿主补丁是额外要求，SDK 版本匹配不能代替此补丁。
