# 文档导航

## 当前维护文档

| 文档 | 用途 |
| --- | --- |
| [项目 README](../README.md) | 核心特性、快速上手和性能测试 |
| [行为契约](spec.md) | 当前命令、输入输出、备注格式、提交规则与边界 |
| [List、编辑与帮助增强规格](list-edit-spec.md) | 已确认的规格快照，实施进度见 [Issue #12](https://github.com/Myraxion/dion/issues/12) |
| [List 与编辑增强设计](list-edit-design.md) | 展示示例与设计调研背景 |
| [领域词汇表](../GLOSSARY.md) | 统一领域术语 |
| [ADR](adr/) | 关键决策及其理由；策略改变时记录替代关系 |
| [Rust 实践](rust-practices.md) | 实现、评审和开发环境规范 |
| [Agent 工作约定](../AGENTS.md) | Issue、分诊标签和领域文档的使用方式，详细说明在 `agents/` |

当前规则以行为契约和 ADR 为依据；功能变化时同步维护相关文档与回归测试。任务状态在 GitHub Issues 中维护。

## 实测报告

- [本地性能基线](benchmarks.md)：特定版本、环境和工作负载下的测量结果及复现方法。
- [TC 双向互操作](tc-interop.md)：真实 TC 验收观察与自动化回归范围。

报告保留历史结果。新版本重新测量时记录版本与环境，另存报告并更新导航；不要用新数据覆盖旧结果，也不要将局部观察扩大为通用保证。

## 历史记录

- [第一版设计](archive/v1-design.md)：开发前确认的产品规则和样本调查。
- [第一版任务拆分](archive/v1-tickets.md)：已结束的实施计划，其中网络共享任务 #9 取消。
- [网络共享实测](archive/unc-validation.md)：已取消支持范围内的历史实验。

归档不表示当前要求、待办或发布门槛。第一版原始需求保留在 [Issue #1](https://github.com/Myraxion/dion/issues/1)；当前行为查阅 [spec.md](spec.md)。
