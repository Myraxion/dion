# 版本查询设计

本文记录 2026-10-14 用户确认并实现的设计。当前行为以 [spec.md](spec.md#版本查询) 为准；下文保留设计理由和验收范围。

## 目的与依据

让用户和脚本确认正在运行的 Dion 程序版本，以便排查 PATH 中的旧程序、核对升级结果和判断版本行为差异。

发布附件统一为 `dion.exe`，文件名无法辨认版本；[v0.3.1 发布记录](releases/v0.3.1.md) 已包含 JSON 破坏性变更，因此确认实际运行版本有具体用途。设计讨论时，现有 Release 程序对 `--version` 返回 `invalid_argument`、退出码 2。

## 已确认的行为

```text
dion [--json|-j] [--lang <en|zh-CN|auto>] --version|-V
```

- 支持 `--version` 和 `-V`，两者等价、区分大小写，遵循现有独立参数规则。
- 成功时 stdout 输出一行 `dion <版本号>`，以换行结束；stderr 为空，退出码为 0。例如当前包版本的输出是 `dion 0.3.1`。
- 版本号取自编译时的 `Cargo.toml` 包版本，通过 `env!("CARGO_PKG_VERSION")` 获取，避免维护第二份版本常量。输出不附加 Git 提交号、构建时间或许可证信息。
- 可搭配 `--json/-j` 和 `--lang`，顺序沿用现有选项规则。成功输出始终为上述文本，语言选择不改变内容，`--json` 不包装 JSON。
- 版本查询仅作为独立入口，不接受命令或其他位置参数，也不接受操作选项。与 `--help/-h` 或 `help` 混用同样报参数错误，不执行帮助或操作。
- 未知、重复选项沿用现有错误规则；`--version -V` 视为重复。`--lang` 的值、缺值及重复校验保持现有规则。
- 参数错误沿用 `invalid_argument`、退出码 2，stdout 为空；错误诊断继续按现有语言及 `--json` 规则输出到 stderr。
- 遵循 `--` 分隔规则，分隔符后的 `--version`、`-V` 是普通位置参数。例如 `dion set target -- --version` 将其视为备注正文。
- 版本查询不读取备注或正文输入，不遍历目录、不修改文件、不启动编辑器。

## 参数组合示例

| 输入 | 结果 |
| --- | --- |
| `dion --version`、`dion -V` | 输出同一行版本文本，退出码 0 |
| `dion --json --lang en -V` | 仍输出版本文本，退出码 0 |
| `dion --version --help` | 参数错误，退出码 2 |
| `dion get target --version` | 参数错误，不读取备注，退出码 2 |
| `dion set target --edit -V` | 参数错误，不启动编辑器或修改备注，退出码 2 |
| `dion --long --version` | 参数错误，退出码 2 |
| `dion --version -V` | 重复选项错误，退出码 2 |
| `dion --version --lang unknown` | 无效语言错误，退出码 2 |

## 实施与验收

- 扩展现有 CLI 解析，在版本查询返回前完成参数校验，包括与帮助入口的互斥校验。
- 同步 `docs/spec.md`、中英文总览帮助及中英文 README，说明入口、版本来源、输出和组合规则。
- 用真实 CLI 验证两个入口、精确输出与退出码、JSON 和语言组合、非法混用、重复选项及 `--` 后的正文和路径语义；确认版本查询及非法混用不读取 stdin、不启动编辑器、不修改备注。
- 在 Windows 执行统一检查入口 `scripts/check.ps1`。

## 验收结果

2026-10-14，Windows 上的 `scripts/check.ps1` 通过格式检查、Clippy、完整 173 项测试和 Release 构建。新增 4 项真实 CLI 测试覆盖版本输出、参数组合、备注文件保持和分隔符语义；既有帮助与语言测试同时通过。Release 程序实测 `--version` 和 `--json --lang zh-CN -V` 均输出 `dion 0.3.1` 加换行。

## 外部约定参考

[GNU 版本选项约定](https://www.gnu.org/prep/standards/html_node/_002d_002dversion.html)支持将版本写入 stdout 并成功退出。它还要求忽略其他参数；本设计采用已确认的严格组合校验，GNU 约定不作为 Dion 的完整行为契约。
