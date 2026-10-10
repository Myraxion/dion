# Dion (descript.ion CLI)

Dion 是一个 Windows 原生命令行工具，用于查看、列出、设置和删除文件与文件夹的备注，与 Total Commander 的带 BOM UTF-8 `descript.ion` 格式互操作。

## 核心特性

- **单文件运行**：适用于 Windows 10/11 x64，无需安装 Rust 或额外运行库；实测 exe 约 346.5 KiB。
- **四个基础命令**：查看单条备注、列出目录备注、设置或覆盖备注、显式删除备注。
- **列表展示**：目录优先、名称自然排序，默认双栏对齐，`--long`（`-l`）单栏展示完整正文，`--recursive`（`-r`）汇总子目录备注，`--tree` 按目录层级展开。
- **Unicode 与多行**：支持中文、Emoji、含空格的名称和多行正文，保留缩进、首尾空白和空行。
- **局部修改**：保留其他记录的原始字节与顺序；设置相同内容时不写文件。
- **正文输入**：支持正文参数、stdin、UTF-8 文本文件及 `set --edit` 外部编辑；沿用 JSON 输出和稳定退出码。

当前支持本地 Windows 文件系统和单写者使用；网络共享不作兼容性承诺。`descript.ion` 必须是带 BOM 的 UTF-8 文件。

名称匹配使用 Windows 原生的不区分大小写比较，不跟随目录的大小写敏感设置；名称等价的重复记录视为冲突。具体规则见[行为契约](docs/spec.md#备注格式与局部修改)。

## 快速上手

### 命令用法

```
dion help [command]
dion [--json] get <path>
dion [--json] set <path> (<comment> | --comment-file <file> | --stdin | --edit)
dion [--json] list [directory] [--long|-l | --tree] [--recursive|-r]
dion [--json] remove <path>
```

### 命令说明

- **`get`**：查看指定文件或文件夹自身的备注。
- **`set`**：设置或覆盖条目自身的备注。非空正文要求目标存在；首次设置自动创建备注文件。正文来源四选一（互斥）：
  - `<comment>`：直接传入正文文本。
  - `--comment-file <file>`：从 UTF-8 文本文件读取（推荐用于多行正文）。
  - `--stdin`：从标准输入读取 UTF-8 正文。
  - `--edit`：启动外部编辑器编辑（依次检索 `VISUAL`、`EDITOR` 或 Windows 记事本；清空内容将删除备注）。
- **`list`**：查询目录内部条目的备注（默认为当前目录）。目录优先并按 Windows 自然排序。
  - 默认双栏对齐展示。
  - `--long`（`-l`）：单栏详细展示，完整呈现多行正文。
  - `--recursive`（`-r`）：递归汇总子目录的备注。
  - `--tree`：树状层级展开展示（自动递归；与 `--long` 互斥）。
  - 默认跳过坏记录或无法读取的备注文件，继续查询；无法枚举的子目录整体跳过，起始目录无法访问时直接失败。
  - 文本底部汇总跳过的错误；JSON 返回 `entries` 和 `errors` 数组，无错误时 `errors:[]`。有跳过错误时仍输出有效记录，退出码为 `1`。
- **`remove`**：显式删除条目自身的备注。目标条目本身不受影响；若目录内备注全部清空，自动清理备注文件。

### 通用选项与特性

- **获取帮助**：支持 `dion help [command]` 以及各命令的 `-h` / `--help` 参数查看用途、用法、参数及关键默认行为。
- **结构化输出**：全局支持 `--json` 输出机器可读的 JSON 格式及稳定退出码。
- **作用对象区分**：`get`、`set`、`remove` 操作指定条目自身的备注（记录保存在其父目录中）；`list` 查询指定目录内部条目的备注。

## 构建与发布

本地安装 Rust stable 和 MSVC 构建工具后，运行统一检查入口：

```powershell
.\scripts\check.ps1
```

脚本依次执行格式检查、Clippy、完整测试和 Release 编译，产物为 `target/release/dion.exe`。默认静态链接 MSVC CRT。

GitHub Actions 在 push 和 pull request 时自动执行上述检查，也支持手动触发。推送 `v*` 版本标签时，[发布流程](.github/workflows/release.yml) 会在 Windows x64 上检查并编译，通过后创建 GitHub Release，自动生成发布说明并上传 `dion.exe`。

发布前更新 `Cargo.toml` 和 `Cargo.lock` 中的版本号，将代码与工作流提交并推送，再推送对应标签，例如：

```powershell
git tag v0.1.0
git push origin v0.1.0
```

用户可在 [GitHub Releases](https://github.com/Myraxion/dion/releases) 下载 `dion.exe`。

## 性能测试

2026-10-08 在 Windows 11 x64、AMD Ryzen 9 7940H、31.2 GB 内存的本地环境中测得以下 Release 基线。每个场景预热后运行 5 次，表中为平均壁钟耗时，包含进程启动和退出；查询与列表使用文本输出。

| 操作 | 100 条记录 | 10,000 条记录 |
| --- | ---: | ---: |
| 查看备注 `get` | 167.73 ms | 148.14 ms |
| 列出备注 `list` | 154.01 ms | 207.29 ms |
| 更新备注 `set` | 167.12 ms | 167.78 ms |
| 删除备注 `remove` | 208.08 ms | 217.87 ms |

本次 exe 体积为 **354,816 字节（346.5 KiB）**；温运行场景的生命周期峰值提交量为 9.26–18.17 MiB。结果仅代表本次机器与工作负载，不构成固定时延保证。完整数据、测量口径与复现方法见[性能测试报告](docs/benchmarks.md)。

## 许可证

可选择 [MIT](LICENSE-MIT) 或 [Apache 2.0](LICENSE-APACHE) 许可证使用和分发。
