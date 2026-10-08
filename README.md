# Dion (descript.ion CLI)

Dion 是一款轻量、高性能、零外部运行依赖的 Windows 原生命令行工具，主用于以 Total Commander 的 **UTF-8 Unicode 编码** 标准查看、设置、编辑与维护 `descript.ion` 文件备注。

第一版实施规格见 [GitHub Issue #1](https://github.com/Myraxion/dion/issues/1)。当前已实现 [Issue #2](https://github.com/Myraxion/dion/issues/2) 的单行备注查询、[Issue #3](https://github.com/Myraxion/dion/issues/3) 的 TC 多行与程序扩展读取、[Issue #4](https://github.com/Myraxion/dion/issues/4) 的目录备注列表、[Issue #5](https://github.com/Myraxion/dion/issues/5) 的单行设置与 Windows 文件提交、[Issue #6](https://github.com/Myraxion/dion/issues/6) 的多行设置与三种输入来源、[Issue #7](https://github.com/Myraxion/dion/issues/7) 的删除备注与最后记录清理及 [Issue #8](https://github.com/Myraxion/dion/issues/8) 的 Windows 长路径、根与链接定位；完整规格包括后续切片，见 [实施规格](docs/spec.md)、[设计规则](docs/design.md) 和 [领域词汇表](GLOSSARY.md)。

开发任务与直接依赖见 [任务索引](docs/tickets.md)，首个任务为 [Issue #2](https://github.com/Myraxion/dion/issues/2)。

## 构建与使用

在 Windows x64 的 Rust MSVC 工具链环境中构建：

```powershell
cargo build --release --locked
.\target\release\dion.exe get '.\照片 😀.txt'
.\target\release\dion.exe get 'D:\资料\文件夹' --json
.\target\release\dion.exe list
.\target\release\dion.exe list 'D:\资料' --json
.\target\release\dion.exe set '.\照片 😀.txt' '  中文备注与字面量\n  ' --json
```

分发 `target/release/dion.exe` 即可；仓库配置静态链接 MSVC CRT，用户无需安装 Rust 或额外运行时。目标平台为 Windows 10/11 x64。

`get` 从输入路径的父目录读取 `descript.ion`，目标文件或文件夹可以不存在。备注文件须为带 BOM 的 UTF-8；接受 CRLF、LF、CR 以及末条无终止换行。含空格的记录名称用双引号包围，名称后第一个空格是分隔符，其后的正文首尾空白原样保留。名称匹配与重复检测使用 Unicode 小写转换，不依赖目录的大小写设置。

四个命令支持本地相对、绝对和 Windows 长路径，包括 `\\?\D:\资料\条目`。路径中的 `.`、`..` 按词法处理，不解析最终链接目标。文件夹自身的 `get/set/remove` 使用父目录备注文件；`list` 使用文件夹内部的备注文件。文件和文件夹符号链接、目录联接的自身备注保存在输入链接条目所在父目录；`list` 链接目录时读取该目录内部的备注。磁盘根可用于 `list`，根自身的 `get/set/remove` 返回参数错误（退出码 2）。

第一版支持范围为本地 Windows 文件系统。UNC、SMB、WebDAV 和网络映射盘不作兼容性承诺，也不属于验收或性能测量要求。现有文件操作仍可能接受这些路径，但不保证可用；程序不主动识别或拒绝网络存储。

文本模式只输出正文，不追加换行。`--json` 可放在命令之前或路径之后，成功输出包含 `name`、`comment`、`extension`（`none`、`tc` 或 `unknown`），保留记录原名称拼写；JSON 为无 BOM 的 UTF-8，以 LF 结束。以 `-` 开头的路径放在 `--` 后，例如 `dion get -- --json`。

错误写入 stderr，正常结果写入 stdout。指定 `--json` 时错误为 `{"error":{"code":"…","message":"…","file":"…","line":1}}`；`file`、物理行号 `line` 仅在适用时提供。完整文件校验通过后才输出结果。

| 退出码 | 含义 | JSON 错误代码 |
| --- | --- | --- |
| 0 | 成功，包含已有空记录 | — |
| 1 | 编码、格式、扩展或文件提交错误 | `invalid_encoding`、`invalid_format`、`unknown_extension`、`content_changed`、`io_error` |
| 2 | 参数错误 | `invalid_argument` |
| 3 | 缺失备注记录或备注文件 | `not_found` |

`list [directory]` 读取指定目录的备注文件，省略目录时使用当前目录。保持记录原始顺序，不检查记录所指条目是否存在，不递归；空记录、孤立记录和未知扩展记录都会列出。文本模式每条记录输出名称加冒号、LF、完整正文及一个用于分隔记录的 LF；JSON 返回 `{"entries":[{"name":"…","comment":"…","extension":"none"}]}`。备注文件不存在、仅有 BOM 或合法空行时成功返回空列表，文本输出为空，JSON 为 `{"entries":[]}`；已有零字节文件因缺少 BOM 报编码错误。目录或备注文件无法访问时返回操作错误。

带 TC UTF-8 扩展标记 `04 C3 82` 的记录将 `\n` 解码为逻辑 LF、`\\` 解码为反斜杠，保留缩进、首尾空白、连续空行和尾部换行数量。无标记记录的反斜杠按字面读取；未知程序扩展只读取控制字符 `04` 前的普通正文，不解释转义。未知反斜杠组合及末尾反斜杠按字面保留，这是 Dion 的产品规则，尚未完整核验 TC 对非标准转义的行为。

`set` 要求目标文件或文件夹存在，每次必须且只能选择正文参数、`--stdin` 或 `--comment-file <file>` 一种来源。三种来源都支持实际多行；参数中的字面量 `\n` 保持字面含义。stdin 和正文文本文件严格使用 UTF-8，接受可选 BOM；`descript.ion` 仍必须有 BOM。CRLF、LF、CR 统一为逻辑 LF，保留缩进、首尾空白、连续空行及末尾换行数量，拒绝空白正文、实际 NUL 和控制字符 `04`。默认成功静默，JSON 返回 `{"changed":true}` 或 `{"changed":false}`。完整文件通过校验、目标没有未知扩展且逻辑正文相同则不写文件。未知扩展即使正文相同也拒绝设置。以 `-` 开头的正文或路径使用 `dion --json set -- <path> <comment>`；流或文件来源可写为 `dion set --stdin -- <path>` 或 `dion set --comment-file <file> -- <path>`。

```powershell
# 从 UTF-8 正文文件输入，不添加或移除末尾换行
.\target\release\dion.exe set '.\照片 😀.txt' --comment-file '.\备注.txt' --json

# Windows PowerShell 5.1 / PowerShell：显式设置发送给原生程序的管道编码
$savedOutputEncoding = $OutputEncoding
try {
    $OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    "中文第一行`n第二行" | .\target\release\dion.exe set '.\照片 😀.txt' --stdin --json
} finally {
    $OutputEncoding = $savedOutputEncoding
}
```

Windows PowerShell 5.1 的默认文本管道可能在内容到达 Dion 前损坏 Unicode，程序无法恢复已经损坏的文字。上例显式选择 UTF-8；PowerShell 文本管道还会追加行终止符，Dion 将其保留为备注末尾的逻辑换行。需要精确保留原文本末尾时使用 `--comment-file`。

首次创建带 BOM 和 CRLF 文件头的隐藏备注文件。实际修改保留目标记录原位置和名称拼写，新增追加末尾；其余内容保留原始字节。多行正文写成一条物理记录，逻辑 LF 编码为 `\n`、反斜杠加倍，并附加 TC 标记 `04 C3 82`；单行正文直接写入，不添加扩展。新写记录以 CRLF 结束，必要时为原末条补 CRLF；完整编码结果包含名称、引号、空格、转义后的正文、扩展与终止符，不得超过 4096 个 UTF-8 字节，超限报错且不落盘。

写入在同目录完整生成临时文件后提交：已有文件使用 Windows `ReplaceFileW`，保留属性、创建时间与访问权限；新文件提交拒绝覆盖后来出现的文件。只读文件的实际修改报错，不解除只读。提交不回退为原地覆盖，也不自动重试或合并。第一版按单写者使用，内容变化检测不提供完整并发保证。成功清理临时文件，提交失败诊断报告备注文件和临时文件路径，保留仍存在的恢复文件；替换可能部分完成，不保证所有 I/O 失败都回滚。被备注条目的属性不变。

`remove <path>` 显式删除整条备注记录，目标文件或文件夹可以不存在；空记录和未知扩展记录都允许删除。保留其他记录的原始字节和顺序，剩余空记录和未知扩展仍计入记录数；删除最后一条实际记录时删除整个备注文件。默认成功静默，JSON 返回 `{"changed":true}`；目标或备注文件不存在时成功返回 `{"changed":false}`，不创建文件，也不清理原本只有 BOM 或空行的文件。坏行和名称冲突导致整次失败，实际删除只读文件报错。多记录更新复用上述临时提交与恢复报告，最后记录清理在只读、占用和内容检查后直接删除文件；均限定单写者，不自动重试。

```powershell
.\target\release\dion.exe remove '.\照片 😀.txt' --json
```

## 开发验证

```powershell
.\scripts\check.ps1
```

脚本依次检查格式、clippy、完整测试和 Release 构建，失败立即退出；Windows CI 使用同一入口。环境排查经验见 [Rust 实践](docs/rust-practices.md#windows-开发环境)。

测试使用可复现的合成备注文件，在本地隔离目录启动真实 CLI 进程，检查输出、退出码、字节及 Windows 属性。TC 互操作与 Release 本地性能基线留给对应后续任务。

Issue #8 的本地黑盒测试已验证超过 260 字符的相对、绝对及扩展路径、Unicode、孤立记录、磁盘根边界、文件夹自身与内部备注区别，以及文件/文件夹符号链接和目录联接实际读写的备注文件。符号链接创建权限不足时测试输出 `UNVERIFIED`，不能据此报告该场景通过。

## 许可证 (License)

本项目遵循双重开源许可协议：
- [MIT 许可证](LICENSE-MIT) ([`LICENSE-MIT`](LICENSE-MIT))
- [Apache 2.0 许可证](LICENSE-APACHE) ([`LICENSE-APACHE`](LICENSE-APACHE))

您可以自由选择任一协议进行使用和分发。
