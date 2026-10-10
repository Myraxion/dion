# Windows 原生名称比较设计

状态：2026-10-10 用户已最终确认共同理解；已实施，通过 Windows 统一检查、性能对比及 Standards/Spec 两路评审。当前已实现行为以[行为契约](spec.md)为准。

## 已确认的决定

- 名称匹配与重复检测统一从 Rust Unicode 小写转换改为 Windows `CompareStringOrdinal(..., TRUE)`，覆盖 `get/set/remove`、外部编辑预填、备注记录重复检测、列表目录识别与树节点合并。
- 继续固定不区分大小写，不读取或跟随目录的大小写敏感设置，沿用 [ADR-0002](adr/0002-case-insensitive-record-names.md) 的范围决定。
- 使用系统大写表进行 ordinal 比较，不受用户语言设置影响，不额外进行 Unicode 规范化；接受与原先 Unicode 小写转换的差异。列表展示继续使用既有 `StrCmpLogicalW` 自然排序及原名称 UTF-16 次级比较。
- 比较规则以该 API 为准，不承诺与所有文件系统或 Total Commander 的名称匹配完全一致。
- 按新比较规则判断重复后，`get/set/remove` 继续整次报 `invalid_format`，`list` 保留首条有效记录、后续重复行报错并返回退出码 1；不自动改名、合并或修复备注文件。
- 重复检测与目录名称索引采用以同一原生比较器定义相等和顺序的 `BTreeSet`，避免逐条两两扫描；集合的比较不加入原名称次级比较。单个目标的记录查找沿用顺序扫描，使用相同的名称相等规则。
- 验收沿用真实 CLI 黑盒测试及既有 Windows 统一检查入口，并比较 100 条与 10,000 条记录的 Release 性能；不新增固定性能门槛。

## 调研事实

- [Microsoft CompareStringOrdinal](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal)：`TRUE` 使用操作系统大写表，支持显式 UTF-16 长度；成功返回小于、等于或大于，失败返回 0。
- [Microsoft ordinal 比较说明](https://learn.microsoft.com/en-us/windows/win32/intl/handling-sorting-in-your-applications)：大小写映射独立于 locale；例如 `å` 与 `a + U+030A` 不合并。
- 本机 Windows `10.0.26100.0` 的原生 P/Invoke 探针显示 `k/K`、`ẞ/ß`、`İ/i + U+0307` 均不相等；实施前的 Rust 小写转换会把这些名称对视为相等。
- 不能假定 `LCMapStringEx` 的 invariant uppercase 映射可生成与该比较器等价的哈希键：本机探针对 Deseret `U+10400/U+10428` 测得 `CompareStringOrdinal(TRUE)` 不相等，但 uppercase 映射相等。探针通过 PowerShell `Add-Type` 调用两个 Kernel32 API，输入采用显式 UTF-16 长度，未改动备注文件。

## 实施范围

- 集中封装 UTF-16 名称及 Windows FFI，传入显式长度，确保名称中的 NUL 不截断比较；调用参数满足 API 要求，沿用项目的错误处理及 `// SAFETY:` 规范。
- 替换 `src/comment.rs` 中的重复检测、设置和删除匹配，`src/cli.rs` 中的查询和编辑预填匹配，以及 `src/listing.rs` 中的目录识别与树节点合并。
- 保留名称原拼写、未修改记录原字节、输出排序及现有保存行为。按 API 返回结果判断名称相等，不加入 locale、Unicode 规范化或另一个大小写映射步骤。
- 实施时更新 `docs/spec.md` 的名称匹配和编辑预填条款，注明既有列表增强设计的 Unicode 小写规则已被本设计取代；不改写历史验收报告中的观察结果。

## 验收范围

- 用真实 CLI 检查普通 ASCII 及非 ASCII 大小写匹配，覆盖读取、设置、删除和编辑预填；设置匹配记录时保留原名称拼写及其他记录字节。
- 检查新旧规则有差异的名称对，覆盖能共存的记录及不会被误匹配的查询；本机探针样例用于复现，最终名称相等规则以运行系统的 API 结果为准。
- 重复记录验证严格命令整次失败且不修改文件，以及 `list` 保留首条、错误行号和退出码的既有行为。
- 列表验证原生比较下的目录识别、目录标记、树节点合并及辅助节点；展示自然排序与原名称次级比较继续符合既有契约。
- 执行 `scripts/check.ps1`，完成格式、clippy、完整测试和 Release 构建。在同一环境对改动前后分别测量 100 条与 10,000 条记录，记录时延、峰值提交量和 exe 体积，保留已有基线报告。

## 实施验证

- 名称相关新增 9 个真实 CLI 回归用例，覆盖独立记录、缺失匹配、写删原字节保留、重复冲突、目录识别、树节点关联、显式长度与 Unicode 组合形式，以及受控编辑器的预填与 no-op。
- 原生 Windows 统一检查通过：格式、clippy、123 个集成测试、Release 构建。新增名称匹配、写入、删除、目录识别、树节点合并和预填用例均先复现旧匹配规则的失败，再通过实现修复。
- [同环境性能对比](windows-name-comparison-benchmarks.md)记录 100 条和 10,000 条记录的五轮温运行、峰值提交量与二进制哈希。改动后 exe 为 493 KiB，比改动前小 10 KiB；本次没有发现规模性时延退化。
- Standards 与 Spec 两路评审均为 0 项发现，评审范围为开始实施前工作区快照到本次实现的增量，未将此前已有的 List 容错改动计为本次新增。
