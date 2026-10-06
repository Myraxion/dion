# Dion Rust 实践

供第一版 Rust 实现与评审使用；产品行为以 [规格](spec.md) 和 [ADR](adr/) 为准。

## 设计与实现

- 遵循 KISS，围绕 CLI 与输出、备注格式、Windows 文件提交组织职责。
- 优先普通函数、具体类型和枚举；trait、泛型及类型状态由实际需求决定。
- 提取函数应表达意图、隐藏复杂度或统一同一规则；允许少量偶然重复。
- 根据 Release 实测优化性能与 exe 体积，先减少多余复制、分配和 I/O。

## 所有权与数据

- 只读参数优先用 `&str`、`&[u8]`、`&Path`；取得所有权时明确使用拥有型参数。
- 按所有权需求选择借用或消费集合，避免循环中冗余 `clone()` 和中间 `collect()`。
- 保留原始文件字节及记录范围，区分物理记录与逻辑正文；局部修改遵循 ADR。
- 严格处理 UTF-8，路径使用路径类型；避免有损转换掩盖非法数据。

## 错误与 Windows 资源

- 可失败操作返回 `Result`，优先用 `?` 传播；用 `Option` 表达正常缺失，保留 I/O 错误。
- 使用结构化错误，在 CLI 边界映射机器错误代码与退出码；文案供人阅读。
- 用户输入及 I/O 失败按错误契约处理，不使用 `panic!`、`unwrap()` 或 `expect()`。
- 用所有权和析构管理文件与句柄；恢复所需临时文件按提交结果保留。
- Windows FFI 与 `unsafe` 集中封装，`// SAFETY:` 说明指针、缓冲区及句柄有效性的依据。

## 测试与文档

- 功能测试运行真实 CLI，在隔离目录检查输出、退出码、文件字节、存在性及属性。
- 每个测试聚焦一个行为，可以有多个断言；测试用户可见结果，保持动作与断言清晰。
- 使用可提交或可复现的样本；真实 UNC、TC 验证及性能测量按规格执行并如实报告。
- 沿用 [领域词汇](../GLOSSARY.md)；`///` 记录接口契约，`//` 解释原因，TODO 引用 issue。

## 检查

建立 Cargo 项目后维护 `Cargo.lock`，执行：

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

lint 例外限定到具体项并说明原因，工具链支持时优先用 `#[expect(...)]`。互斥 feature 分组合检查。

## 按需参考

格式、编码、输出、提交及性能验收细节查阅 [规格](spec.md) 的相关章节和 [ADR](adr/)；通用实践参考 Apollo GraphQL 的 [Rust Best Practices Handbook](https://github.com/apollographql/rust-best-practices)。
