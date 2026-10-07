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

维护 `Cargo.lock`，在 Windows 执行统一检查入口：

```powershell
.\scripts\check.ps1
```

该脚本依次执行格式、clippy、完整测试和 Release 构建，任一失败立即退出并返回该命令的退出码。[Windows CI](../.github/workflows/check.yml) 使用同一入口，在 push 和 pull request 时自动运行，也支持手动触发。

lint 例外限定到具体项并说明原因，工具链支持时优先用 `#[expect(...)]`。互斥 feature 分组合检查。

## Windows 开发环境

- CLI 测试在系统临时目录中创建隔离文件。若 Agent 沙箱报路径规范化失败或访问被拒绝，先判断限制来自沙箱还是产品权限；需要时按授权流程在沙箱外重跑同一命令。一次沙箱失败不能作为产品权限行为的测试结论。
- 从其他 PowerShell 版本启动 `powershell.exe` 时，继承的 `PSModulePath` 可能导致 `Get-Acl` 所需模块加载失败或类型数据重复。测试子进程清除该环境变量，让 Windows PowerShell 使用自身的模块路径；现有示例见 `tests/set.rs` 的 `powershell` 辅助函数。
- `get`、`set` 的共同路径行为在 `tests/paths.rs` 验证，包括当前目录、点组件及相对/绝对路径。实现 `remove` 时沿用相同用例，仍通过真实 CLI 验证父目录定位和链接条目语义。

## 按需参考

格式、编码、输出、提交及性能验收细节查阅 [规格](spec.md) 的相关章节和 [ADR](adr/)；通用实践参考 Apollo GraphQL 的 [Rust Best Practices Handbook](https://github.com/apollographql/rust-best-practices)。
