# Dion (descript.ion CLI)

Dion 是一款轻量、高性能、零外部运行依赖的 Windows 原生命令行工具，主用于以 Total Commander 的 **UTF-8 Unicode 编码** 标准查看、设置、编辑与维护 `descript.ion` 文件备注。

第一版实施规格见 [GitHub Issue #1](https://github.com/Myraxion/dion/issues/1)。当前已实现 [Issue #2](https://github.com/Myraxion/dion/issues/2) 的单行备注查询及 [Issue #3](https://github.com/Myraxion/dion/issues/3) 的 TC 多行与程序扩展读取；完整规格包括后续切片，见 [实施规格](docs/spec.md)、[设计规则](docs/design.md) 和 [领域词汇表](GLOSSARY.md)。

开发任务与直接依赖见 [任务索引](docs/tickets.md)，首个任务为 [Issue #2](https://github.com/Myraxion/dion/issues/2)。

## 构建与使用

在 Windows x64 的 Rust MSVC 工具链环境中构建：

```powershell
cargo build --release --locked
.\target\release\dion.exe get '.\照片 😀.txt'
.\target\release\dion.exe get 'D:\资料\文件夹' --json
```

分发 `target/release/dion.exe` 即可；仓库配置静态链接 MSVC CRT，用户无需安装 Rust 或额外运行时。目标平台为 Windows 10/11 x64。

`get` 从输入路径的父目录读取 `descript.ion`，目标文件或文件夹可以不存在。备注文件须为带 BOM 的 UTF-8；接受 CRLF、LF、CR 以及末条无终止换行。含空格的记录名称用双引号包围，名称后第一个空格是分隔符，其后的正文首尾空白原样保留。名称匹配与重复检测使用 Unicode 小写转换，不依赖目录的大小写设置。

文本模式只输出正文，不追加换行。`--json` 可放在命令之前或路径之后，成功输出包含 `name`、`comment`、`extension`（`none`、`tc` 或 `unknown`），保留记录原名称拼写；JSON 为无 BOM 的 UTF-8，以 LF 结束。以 `-` 开头的路径放在 `--` 后，例如 `dion get -- --json`。

错误写入 stderr，正常结果写入 stdout。指定 `--json` 时错误为 `{"error":{"code":"…","message":"…","file":"…","line":1}}`；`file`、物理行号 `line` 仅在适用时提供。完整文件校验通过后才输出结果。

| 退出码 | 含义 | JSON 错误代码 |
| --- | --- | --- |
| 0 | 成功，包含已有空记录 | — |
| 1 | 编码、格式或文件访问错误 | `invalid_encoding`、`invalid_format`、`io_error` |
| 2 | 参数错误 | `invalid_argument` |
| 3 | 缺失备注记录或备注文件 | `not_found` |

带 TC UTF-8 扩展标记 `04 C3 82` 的记录将 `\n` 解码为逻辑 LF、`\\` 解码为反斜杠，保留缩进、首尾空白、连续空行和尾部换行数量。无标记记录的反斜杠按字面读取；未知程序扩展只读取控制字符 `04` 前的普通正文，不解释转义。未知反斜杠组合及末尾反斜杠按字面保留，这是 Dion 的产品规则，尚未完整核验 TC 对非标准转义的行为。`list`、`set`、`remove` 尚未实现。

## 开发验证

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

测试使用可复现的合成备注文件，在隔离目录启动真实 CLI 进程，检查输出、退出码、字节及 Windows 属性。真实 UNC、TC 互操作与 Release 性能基线留给对应后续任务。

## 许可证 (License)

本项目遵循双重开源许可协议：
- [MIT 许可证](LICENSE-MIT) ([`LICENSE-MIT`](LICENSE-MIT))
- [Apache 2.0 许可证](LICENSE-APACHE) ([`LICENSE-APACHE`](LICENSE-APACHE))

您可以自由选择任一协议进行使用和分发。
