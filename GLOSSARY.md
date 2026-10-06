# Dion

Dion 的领域是保存在 `descript.ion` 中的文件与文件夹备注。

## Language

**备注（Comment）**：
用户为某个文件或文件夹附加的文字说明，可以包含多行。
_Avoid_: 标签、NTFS 元数据

**备注文件（Description file）**：
保存同一目录下文件与文件夹备注的 `descript.ion` 文件。

**备注记录（Comment record）**：
备注文件中将一个文件或文件夹名称与其备注关联起来的一条记录。

**孤立记录（Orphan record）**：
备注文件中所指文件或文件夹已经不存在的备注记录。

**空记录（Empty record）**：
文件或文件夹名称存在于备注文件中，但关联的备注正文为空的记录。
_Avoid_: 无记录

**程序扩展（Program extension）**：
备注记录中由控制字符 `04` 引入、供特定程序使用的信息。

**逻辑换行（Logical line break）**：
备注正文中相邻文本行之间的分隔，区别于备注文件内结束一条记录的物理换行。
