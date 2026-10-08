# Dion (descript.ion CLI)

Dion 是一款轻量、高性能、零外部运行依赖的 Windows 原生命令行工具，主用于以 Total Commander 的 **UTF-8 Unicode 编码** 标准查看、设置、编辑与维护 `descript.ion` 文件备注。

第一版实施规格见 [GitHub Issue #1](https://github.com/Myraxion/dion/issues/1)。第一版规划的 10 个切片开发任务已全部交付完成（[Issue #2](https://github.com/Myraxion/dion/issues/2) 至 [Issue #11](https://github.com/Myraxion/dion/issues/11)，其中 #9 网络共享按用户决定取消，#10 完成真实 TC 双向互操作验收，#11 完成 Release 单 exe 与本地性能基线）；完整规格包括后续切片，见 [实施规格](docs/spec.md)、[设计规则](docs/design.md)、[任务索引](docs/tickets.md) 和 [领域词汇表](GLOSSARY.md)。

## 构建与分发

在 Windows x64 的 Rust MSVC 工具链环境中构建：

```powershell
cargo build --release --locked
.\target\release\dion.exe get '.\照片 😀.txt'
.\target\release\dion.exe get 'D:\资料\文件夹' --json
.\target\release\dion.exe list
.\target\release\dion.exe list 'D:\资料' --json
.\target\release\dion.exe set '.\照片 😀.txt' '  中文备注与字面量\n  ' --json
```

分发 `target/release/dion.exe` 单文件即可：
- **零外部运行依赖**：仓库配置静态链接 MSVC CRT（`+crt-static`），目标平台为 Windows 10/11 x64，用户无需安装 Rust、Visual C++ 运行库或任何额外环境。
- **轻量体积**：暂以体积不超过 5 MiB 为发布目标；实测 Release 单 exe 体积仅为 **354,816 字节**（约 346.5 KiB，**0.34 MiB**），远低于目标上限。

## 本地性能基线

在 Windows 11 x64 本地临时目录中，使用 Release 构建分别测量 100 条与 10,000 条记录下的 get、list、set、remove。2026-10-08 修正测量脚本后重跑的环境、机器配置、工作负载与完整数据见 [本地性能基线报告](docs/benchmarks.md)。

- **首次调用**：空目录 `list` 壁钟耗时 170.94 ms，生命周期峰值提交量 9.24 MiB。脚本不清空系统文件缓存，首次调用不代表缓存冷启动。
- **100 条记录（5,330 字节）**：预热后，各场景平均壁钟耗时 153.82–208.08 ms，平均 CPU 耗时 106.25–121.88 ms，峰值提交量 9.26–9.35 MiB。
- **10,000 条记录（531,350 字节）**：预热后，各场景平均壁钟耗时 148.14–217.87 ms，平均 CPU 耗时 106.25–162.50 ms，峰值提交量 11.02–18.17 MiB。
- **统计口径**：温运行每个场景测量 5 次；上述区间是各场景统计值的最小值与最大值，不是性能保证。通过保留的进程句柄查询生命周期峰值提交量，非工作集；旧 Job 测量结果不直接用于比较。存储介质类型未人工核验。
- **时延预算原则**：基线取得前不承诺固定耗时门槛；产出可复现结果供后续确认时延预算，不将未经验证的新阈值暗中加入规格。

在 PowerShell 7 中复现基准及检查测量脚本：

```powershell
cargo build --release --locked
.\scripts\benchmark.ps1
.\scripts\test-benchmark.ps1
```

## 使用说明

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

写入在同目录完整生成临时文件后提交：已有文件使用 Windows `ReplaceFileW`，保留属性、创建时间与访问权限；新文件提交拒绝覆盖后来出现的文件。只读文件的实际修改报错，不解除只读。提交不回退为原地覆盖，也不自动重试或合并。第一版按单写者使用，内容变化检测不提供完整并发保证。默认不生成常驻备份，也不提供备份选项；成功清理临时文件，提交失败诊断报告备注文件和临时文件路径，保留仍存在的恢复文件；替换可能部分完成，不保证所有 I/O 失败都回滚。被备注条目的属性不变。

`remove <path>` 显式删除整条备注记录，目标文件或文件夹可以不存在；空记录和未知扩展记录都允许删除。保留其他记录的原始字节和顺序，剩余空记录和未知扩展仍计入记录数；删除最后一条实际记录时删除整个备注文件。默认成功静默，JSON 返回 `{"changed":true}`；目标或备注文件不存在时成功返回 `{"changed":false}`，不创建文件，也不清理原本只有 BOM 或空行的文件。坏行和名称冲突导致整次失败，实际删除只读文件报错。多记录更新复用上述临时提交与恢复报告，最后记录清理在只读、占用和内容检查后直接删除文件；均限定单写者，不自动重试。

```powershell
.\target\release\dion.exe remove '.\照片 😀.txt' --json
```

## 范围外功能 (Out of Scope)

依据第一版实施规格，以下功能明确属于第一版范围外：

- **网络共享与非本地存储**：UNC 路径、SMB 共享、WebDAV 和网络映射盘的兼容性承诺与专用支持；
- **非 UTF-8 编码**：ANSI/OEM、UTF-16 读写或自动转换，以及接受无 BOM 的 `descript.ion`；
- **高级查询与维护**：递归查询、备注全文搜索、批量导入导出、自动清理孤立记录、坏文件自动修复；
- **文件联动**：文件复制、移动、重命名及对应的备注同步更新；
- **其他形态**：NTFS 备注转换、文件系统变动监控、后台服务、图形界面或交互式编辑器；
- **常驻备份与多写者并发**：不生成常驻备份文件或提供备份选项；不提供多写者、跨机器并发协调与自动重试；
- **未知扩展改写**：不解读或改写未知程序扩展（仅支持读取前置普通正文和整条记录显式删除）；
- **未经测量的时延门槛**：基线确立前不暗自加入固定时延要求。

## 开发验证

```powershell
.\scripts\check.ps1
```

脚本依次检查格式、clippy、完整测试和 Release 构建，失败立即退出；Windows CI 使用同一入口。环境排查经验见 [Rust 实践](docs/rust-practices.md#windows-开发环境)。

测试使用可复现的合成备注文件与真实 TC 文件，在本地隔离目录启动真实 CLI 进程，全面检查输出、退出码、原始字节及 Windows 属性契约。全套 75 项黑盒集成测试已在 Debug 与 Release 优化编译下全部通过。

- **真实 TC 双向互操作**：已完成真实 Total Commander（UTF-8 编码设置）与 Dion CLI 的双向读写验收，详见 [TC 互操作实测报告](docs/tc-interop.md)。明确区分了 Dion 产品规则与实际 TC 观察（如删除最后记录自动删除文件、末条无换行、4096 字节含终止符、未知反斜杠按字面保留等）。
- **未验证场景与权限边界**：符号链接创建需 Windows 开发者模式或提升权限，在权限不足时测试输出 `UNVERIFIED`，严格区分未验证状态，不虚报通过。网络存储（UNC/SMB）不提供支持承诺，不包含在第一版验收范围内。

## 许可证 (License)

本项目遵循双重开源许可协议：
- [MIT 许可证](LICENSE-MIT) ([`LICENSE-MIT`](LICENSE-MIT))
- [Apache 2.0 许可证](LICENSE-APACHE) ([`LICENSE-APACHE`](LICENSE-APACHE))

您可以自由选择任一协议进行使用和分发。
