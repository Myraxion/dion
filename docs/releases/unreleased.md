# 未发布变更

## JSON 破坏性变更

- `list --json` 的每条 `entries` 记录只保留 `name`、`comment`，删除 `extension`；顶层 `entries`、`errors` 数组及退出码规则不变。普通、单栏、递归、树状和 Everything 查询统一使用此结构；未知程序扩展记录仍列出已解析的正文。
- `get --json` 将 `extension` 改名为 `program_extension`，值仍为 `none`、`tc`、`unknown`。旧字段不再输出，依赖它的脚本应更新字段名；原先通过列表检查程序扩展类型的脚本需要对条目调用 `get --json`。

新结构示例：

```json
{"entries":[{"name":"a.txt","comment":"备注正文"}],"errors":[]}
```

```json
{"name":"a.txt","comment":"备注正文","program_extension":"none"}
```
