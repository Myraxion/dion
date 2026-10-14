# JSON 列表显式表达记录归属与实际目录层级

2026-10-13，用户确认所有 `list` JSON 条目增加 `type`（`file`、`directory`、`orphan`），普通及递归扁平列表将记录所属目录放入 `parent_directory`，`name` 只保留备注记录原始名称；所属目录相对于查询起始目录，使用 Windows 反斜杠且无尾部分隔符，起始目录为 `""`。名称可能包含分隔符或点组件，将路径拼入名称会使 agent 无法准确区分记录归属与名称文本，因此直接调整旧契约，不提供兼容别名或开关。

`--tree --json` 改为嵌套树，复用文本树的节点选择、目录备注合并和每层排序，层级只来自实际目录遍历，不拆分记录名称，不显示起始目录根节点。节点含 `type`、`name`、`comment`，仅目录节点始终带 `children`（可为空）；辅助目录使用实际目录名及 `comment: null`，已有空备注使用 `comment: ""`。树节点不含 `parent_directory`，两种列表均不含 `extension` 或 `program_extension`，沿用 [ADR-0005](0005-json-program-extension.md) 的程序扩展字段决定。

两种输出顶层包含 `root_directory`、`entries`、`errors`。`root_directory` 是查询起始目录的绝对路径，按现有路径规则词法处理点组件，不解析符号链接或 Junction 的实际目标，保留路径拼写和 Windows 扩展路径前缀；空列表和含跳过错误的结果也包含该字段，使结果脱离命令调用上下文后仍能确定记录归属。扁平列表不生成辅助目录节点，文件和孤立记录仍混合参与文件组自然排序。文本输出、递归过滤、目录链接边界、错误及退出码规则不变。Everything 查询使用同一输出契约，保留其索引覆盖和诊断范围差异。
