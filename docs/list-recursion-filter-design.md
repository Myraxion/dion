# List 递归目录排除设计

本文记录 2026-10-11 确认的设计及实现结果；当前行为契约以 [spec.md](spec.md) 为准。

## 已确认

- 默认排除限制递归进入目录，不隐藏父目录备注文件中对该目录的备注记录。
- 普通非递归 `list` 的备注查询范围不变；新规则用于 `--recursive` 和 `--tree` 的递归遍历。
- 默认不进入的目录名单：`$RECYCLE.BIN`、`System Volume Information`、`.git`、`node_modules`、`.venv`、`__pycache__`、`.pytest_cache`、`.next`、`.svn`、`.mypy_cache`、`.ruff_cache`、`.tox`、`.nox`、`.parcel-cache`。
- 用户显式指定名单中的目录作为起始目录时，仍尝试读取；权限不足沿用现有错误规则。起始目录内部的递归子目录仍适用默认排除规则。
- 名单在任意递归层级适用，按目录的完整名称、不区分大小写匹配，沿用现有 Windows 名称比较规则。不使用前缀、通配符或 Hidden/System 属性过滤；`.GIT` 匹配 `.git`，`.git-backup` 不匹配。
- 增加 `list --all`（`-a`），取消上述名单的默认排除，可与 `--recursive` 或 `--tree` 组合。此选项不改变递归中不进入目录符号链接和 Junction 的既有规则。
- 按名单主动跳过是预期行为，不生成诊断，不加入 JSON 的 `errors`，不因此返回退出码 1。实际遇到的权限、编码、格式等错误沿用现有错误规则。
- 文本与 JSON 使用相同查询范围；父目录中被排除目录自身的备注记录仍正常显示，树状输出继续沿用既有辅助目录节点规则。
- 未开启递归时也接受 `--all`（`-a`），结果与普通 `list` 相同；该选项不自动开启递归，递归仍由 `--recursive`（`-r`）或 `--tree` 开启。
- `--all`（`-a`）仅适用于 `list`，沿用现有布尔选项的解析与校验约定。
- 实现完成：默认递归排除、`--all`（`-a`）、中英文帮助、README、行为契约及真实 CLI 集成测试均已同步。

## 调研依据与范围说明

- `System Volume Information` 由 Windows 在卷上创建，带 Hidden 和 System 属性；NTFS 上的创建逻辑为 SYSTEM 配置访问控制。
- `$WINDOWS.~BT` 可作为后续候选：Windows Setup 在其中提取安装源文件；当前已确认名单不包含它。
- 不建议默认排除 `Windows.old`：它可能保留升级前的用户文件，因此也可能保留用户备注。
- 不额外排除 `Windows`、`Program Files`、`ProgramData`、`AppData` 或名单以外的项目依赖目录。

## 实施与验收

- 在普通递归与树状列表共用的遍历逻辑中，仅过滤待进入的子目录，保留目录类型识别与父目录中的全部有效备注记录。
- 更新 CLI 解析、中英文帮助、中英文 README 和 `docs/spec.md`，说明默认名单及 `--all`（`-a`）。
- 真实 CLI 集成测试覆盖默认名单、任意层级及大小写匹配、非匹配名称、目录自身备注保留、显式起始目录、主动跳过不报错、`--all` 恢复查询、非递归 `list -a` 及树状列表。
- 完整检查使用 `scripts/check.ps1`。

## 调研依据

- [Windows 回收站诊断](https://learn.microsoft.com/en-us/troubleshoot/windows-client/shell-experience/recycle-bin-corrupted)
- [RtlCreateSystemVolumeInformationFolder](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-rtlcreatesystemvolumeinformationfolder)
- [SetupDiag](https://learn.microsoft.com/en-gb/windows/deployment/upgrade/setupdiag)
- [从 Windows.old 找回用户文件](https://support.microsoft.com/en-us/windows/deployment/install-upgrade/retrieve-files-from-the-windows-old-folder-after-a-windows-upgrade)
- 当前实现 `src/listing.rs` 与当前契约 `docs/spec.md`：仅限制目录链接与 Junction 的递归进入，没有名称、Hidden 或 System 属性过滤；错误跳过后仍返回退出码 1。
