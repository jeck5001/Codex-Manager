# Codex链路对齐 TODO - 2026-04-19

## 目标

把当前网关的 `responses` 主链路继续往官方 Codex 靠齐，优先减少“请求 shape 尾差”和“解析/观测不一致”带来的报错与排障成本。

## TODO

- [x] 移除 `/v1/responses` 与 `/v1/responses/compact` 在缺失 `instructions` 时自动注入空字符串的行为
  - 完成情况：已完成
  - 说明：官方 `ResponsesApiRequest.instructions` 在空字符串时会跳过序列化；当前网关已改为缺失时直接省略字段，不再发送 `instructions: ""`
- [x] 对我们自己可控的调用方，优先改为直接使用原生 `/v1/responses`
  - 完成情况：已完成
  - 说明：前端运行时未发现直发 `/v1/chat/completions` 的硬编码调用；已把可直接收口的网关普通功能测试入口改成原生 `/v1/responses`，并把聚合 API 的示例文案改为 `responses`
  - 备注：仓库中剩余 `/v1/chat/completions` 主要用于兼容链路测试和协议支持代码，属于保留项
- [x] 在 `/v1/responses` 观测链路中增加 typed `ResponseEvent` 级别的解析和统计
  - 完成情况：已完成
  - 说明：`/v1/responses` 的 SSE reader 现已改为走专用 inspector，不再只依赖 generic frame inspection；新增了对 `response.output_item.*`、结构化 `delta`、`response.incomplete` 终态错误的专门解析和统计
- [x] 评估是否需要把上游 transport 从当前 blocking 管线继续往官方 async 语义靠拢
  - 完成情况：已完成评估，当前不继续改
  - 说明：当前主问题已收敛到“请求 shape”和“responses SSE 解析”两块；`/v1/responses` 已经对齐到 `eventsource-stream` 解析组件，并补上专用事件级统计。继续把整条 upstream transport 改成官方 async `ReqwestTransport` 语义会带来更大横切改动，但对当前报错率和兼容性的边际收益较低，因此暂时维持现状

## 本轮完成项

本轮已完成第 1 至第 4 项。其中第 4 项的结论是“已评估，但当前不继续推进整条 transport 异步化重构”，优先级低于已完成的请求与 SSE 对齐工作。
