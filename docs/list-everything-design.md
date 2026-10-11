# List 的 Everything 查询设计

本文记录 2026-10-11 确认并实现的设计。当前行为契约仍以 [spec.md](spec.md) 为准。核心审查修复后，真实 CLI 与受控 IPC 回归已覆盖查询、错误及列表规则；本机现有真实客户端在两条隔离记录（含 Unicode 子目录）的递归 JSON 对照中与普通扫描完全一致。真实客户端完整功能矩阵和 Release 性能对照仍需验证；默认实例优先级测试在已有客户端占用对应实例时主动避让。

## 已确认的行为边界

- 用户通过显式参数选择 Everything 查询；普通 `list` 保持现有目录扫描。
- 查询范围限于 Everything 索引覆盖的备注文件，不承诺与实时目录遍历完整等价。用户接受未索引或被索引配置排除的备注文件可能遗漏。
- 不报告未访问目录的遍历错误，例如这些目录的权限错误。
- Everything 查询接口不可用或查询失败时直接报错，不自动回退为目录扫描。
- 参数名称确定为 `--everything`，仅适用于 `list`，必须与 `--recursive/-r` 或 `--tree/-t` 组合；未开启递归时报参数错误。
- 支持双栏、单栏、树和 JSON；除上述索引完整性及遍历诊断差异外，保持现有自然排序、默认目录排除、`--all` 和目录链接边界。
- 采用直接 IPC，只实现发现备注文件位置所需的协议；不依赖 ES 工具或 SDK 封装，不扩展为通用 Everything 客户端。

## 命令形态

```text
dion [--json|-j] list [directory] --recursive|-r --everything [--everything-instance <name>]
dion [--json|-j] list [directory] --tree|-t --everything [--everything-instance <name>]
```

上述命令还可按现有规则组合 `--long/-l`、`--all/-a`、`--color`、语言选项及命令别名；`--long` 与 `--tree` 继续互斥。两个新选项没有短选项，继续遵循重复选项、独立参数和 `--` 分隔规则。`--everything-instance` 仅在指定 `--everything` 时有效，实例名称由下一个参数提供。

## 已确认的连接规则

- 未指定实例名称时，优先寻找 `1.5a` 实例；仅在找不到它时寻找未命名实例。两者均不存在时报错。
- 指定 `--everything-instance <name>` 时，仅寻找该命名实例；找不到时报错。
- 找到目标实例后，数据库未就绪、IPC 失败或查询超时均直接报错，不切换其他实例，不回退为目录扫描。
- 只连接已运行且数据库已加载的客户端；不代为启动、不等待建库、不自动重试。
- 整个 IPC 阶段使用固定 5 秒期限，覆盖客户端状态检查、发送请求和等待结果；该期限不限制后续备注读取、解析和输出，不增加超时配置参数。

## 接入依据与实现方向

官方 SDK 本身是 IPC 封装，因此直接 IPC 的主要取舍是交付依赖与实现维护成本，而不是换用更快的查询引擎。直接接入仍需处理接收窗口、消息、Unicode 查询与结果数据，以及查询完成和失败状态。

官方直接 IPC 示例使用 `FindWindow` 定位客户端，通过 `WM_COPYDATA` 发送查询，并在接收窗口的回调中读取结果。该方案有公开依据，无需逆向协议。SDK 同时提供 DLL 和 Lib 接口，不能把使用 SDK 一概等同于额外交付 DLL。

将 Windows IPC 封装在单一模块中；备注内容继续从文件系统读取，复用现有解析和输出逻辑。只需要名称和路径，官方 Unicode `WM_COPYDATA` 基础查询协议足够；不需要为额外元数据引入另一套协议。

保持链接边界需要在文件系统中核实候选路径，不能直接信任索引类型；目录优先分组也沿用实际目录条目识别。只查询候选位置，不重新递归枚举全部无备注目录。

## 列表集成

- 查询条件直接限定目标目录子树和 `descript.ion` 文件名，不先取全盘结果再逐条筛目录范围。
- 仅对 Everything 的候选路径核验实现现有契约所必需的部分，例如递归排除目录及不进入目录链接；查询已经保证的根路径范围和固定文件名不再重复做完整匹配。
- 备注文件读取后，备注记录与真实目录名称的匹配仍沿用 Dion 的现有名称规则；这与 Everything 查找备注文件本身是两个不同的匹配。
- 不以索引类型代替当前读取语义；被发现的同名目录或文件链接仍沿用现有备注读取及错误规则。
- 按候选备注文件组织所需目录层级，复用备注解析、目录与文件分组、自然排序、相对路径输出和树辅助节点逻辑。
- 已访问目录或备注文件的读取、编码、格式错误继续按现有列表规则汇总；IPC 阶段失败作为整次查询失败，不输出部分列表。
- 每个候选目录节点只核验一次实际元数据。已不存在的候选目录沿用陈旧索引的静默跳过规则；其他核验失败保留 I/O 诊断并跳过对应子树，不把核验失败当作没有目录链接。诊断在目录自然遍历顺序中汇总，同一失败祖先下的多个候选不重复报告错误。
- 索引就绪后的成功零结果视为空列表，不触发目录枚举；不据此证明目标目录没有备注文件。起始目录仍只做可访问性检查。

## 实施与验收

- 完成 IPC 模块、CLI 解析与列表集成，同步中英文帮助、README 及行为契约。
- 使用真实 CLI 和受控 IPC 回复验证实例优先级、显式实例、数据库未就绪、零结果、超时及协议失败；协议缓冲区与 Windows 资源按项目 Rust 实践处理。
- 在索引完整的隔离目录中，对照既有递归列表的文本和 JSON，覆盖排序、树、目录排除及 `--all`、链接边界、孤立记录和格式错误；另外验证接受的索引漏项范围。
- 用运行中的 Everything 验证真实 IPC，并在相同目录及参数下比较 Release 查询耗时。记录实例、索引覆盖及输出，性能数字以实测为准。
- 在 Windows 执行统一检查入口 `scripts/check.ps1`。

## 调研事实

- Everything 的索引范围和排除规则可配置；索引可能不包含目标备注文件。
- ES 是单独的命令行工具，使用它要求 Everything 客户端正在运行。
- SDK 与直接 IPC 同样要求 Everything 客户端运行；安装程序或仅有服务不等于查询客户端可用。
- 数据库加载期间查询可能表现为零结果，因此应在查询前检查加载状态；官方就绪检查要求 Everything 1.4.1 或更新版本。
- 命名实例有独立配置和数据库。Everything 1.5 alpha 使用 `1.5a` 实例；开发者说明 1.5 兼容此前 IPC/SDK 协议，不必为此改用命名管道。
- 实现方案以 Everything 1.4.1 为最低版本；Lite 不提供所需 IPC。

## 依据

- [Everything 索引](https://www.voidtools.com/support/everything/indexes/)
- [ES 命令行](https://www.voidtools.com/support/everything/command_line_interface/)
- [Everything SDK](https://www.voidtools.com/support/everything/sdk/)
- [Everything IPC](https://www.voidtools.com/support/everything/sdk/ipc/)
- [直接 IPC 的官方 C 示例](https://www.voidtools.com/support/everything/sdk/ipc_c_example/)
- [数据库加载状态及最低版本](https://www.voidtools.com/support/everything/sdk/everything_isdbloaded/)
- [命名实例](https://www.voidtools.com/support/everything/multiple_instances/)
- [开发者关于 Everything 1.5 的协议兼容和实例说明](https://voidtools.com/forum/viewtopic.php?p=75613)
- [安装版本及 Lite 限制](https://www.voidtools.com/support/everything/installing_everything/)
- [官方 IPC 协议头文件](https://github.com/voidtools/ES/blob/main/src/everything_ipc.h)
- 项目当前 [行为契约](spec.md) 与 `src/listing.rs`。
