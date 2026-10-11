# 文档导航

本文档整理 Dion 项目的文档体系。当前开发已基本完成并正常投入使用，所有外部行为规则与回归测试均以**核心规范**为唯一依据。

## 核心规范与准则（唯一事实源 SSOT）

以下文档定义 Dion 当前的外部行为契约、领域模型与工程规范。**新增特性、修改既有逻辑或编写回归测试时，必须以此分类下的文档为准。**

| 文档 | 用途 |
| --- | --- |
| [行为契约](spec.md) | **系统当前外部行为的唯一权威标准**：包含命令形态、输入输出、列表排版、终端着色、外部编辑、备注编码、提交语义及错误代码 |
| [领域词汇表](../GLOSSARY.md) | 统一领域命名与术语边界 |
| [架构决策记录 (ADR)](adr/) | 关键技术决策及其背景理由；架构或策略变更时记录替代关系 |
| [Rust 开发实践](rust-practices.md) | 编码风格、所有权、错误映射、Windows FFI 安全规范与统一检查说明 |
| [Agent 工作约定](../AGENTS.md) | Issue 管理、分诊标签及领域文档约定，具体指南见 [agents/](agents/) |
| [项目 README](../README.md) | 用户面向的产品概览、快速上手与命令行参考（英文版见 [README.en.md](../README.en.md)） |

## 实测与基线报告

记录特定版本和环境下的性能基线与跨工具兼容性实测数据。新版本重测时另存新报告，不覆盖历史数据。

- [本地性能基线 (v0.1.0)](benchmarks.md)：100 条与 10,000 条记录下的冷启动、温运行耗时、峰值提交量与复现方法。
- [Windows 名称比较性能对比](windows-name-comparison-benchmarks.md)：原生名称比较改动前后的同环境对比测量与产物 SHA-256。
- [TC 双向互操作实测报告](tc-interop.md)：与 Total Commander 11.58 真实双向交互、编码往返与边界用例实测。

## 版本发布记录

- [v0.2.0 发布记录](releases/v0.2.0.md)：列表容错跳过、命令别名与短选项、终端着色、中英文多语言、Windows 原生名称比较及树状连接线修复。

## 历史归档与调研记录 (archive/)

以下文档已全量实现或归档，仅作为技术调研背景与历史演进记录留存。**其有效规则已全部并入 [行为契约](spec.md)**；归档内容不表示当前要求或待办事项，**严禁依据归档中的早期草案覆盖或修改既有契约**。

### 已交付特性设计与调研档案

- [Windows 原生名称比较设计](archive/windows-name-comparison-design.md)：`CompareStringOrdinal` 系统大写表、Unicode 组合字符探针、API 等价边界及性能验收。
- [多语言支持设计](archive/multilingual-design.md)：`GetUserDefaultUILanguage` 回退规则、中英双语诊断及全局 `--lang` 参数优先级。
- [List 名称着色设计](archive/list-color-design.md)：Windows 控制台 VT 模式检测、`NO_COLOR` 支持、SGR 36 终端青色与无色回退方案。
- [命令别名与参数简写设计](archive/cli-alias-design.md)：`view`/`cat`、`ls`、`rm` 等命令别名与短选项规范。
- [List 与外部编辑增强设计](archive/list-edit-design.md)：双栏、单栏、树状多行垂直引导线及外部编辑器冲突检测设计背景。
- [List、编辑与帮助增强规格快照](archive/list-edit-spec.md)：[Issue #12](https://github.com/Myraxion/dion/issues/12) 开发期规格快照（已封存）。

### 历史版本规划与取消范围

- [第一版设计](archive/v1-design.md)：v0.1.0 开发前确认的产品规则和样本调查。
- [第一版任务拆分](archive/v1-tickets.md)：v0.1.0 实施计划，其中网络共享任务 #9 取消。
- [网络共享实测](archive/unc-validation.md)：已取消支持范围内的历史实验。

归档与历史快照不代表当前要求或待办事项。当前行为请始终查阅 [spec.md](spec.md)。
