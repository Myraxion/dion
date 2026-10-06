# Dion 第一版开发任务

来源：[规格 Issue #1](https://github.com/Myraxion/dion/issues/1)。经用户确认的 10 个任务已于 2026-10-06 发布，发布时均标记 `ready-for-agent`；14 条直接依赖已设置为 GitHub 原生阻塞关系。

| Issue | 任务 | 直接阻塞项 |
| --- | --- | --- |
| [#2](https://github.com/Myraxion/dion/issues/2) | 单行备注查询与 CLI 黑盒测试入口 | 无 |
| [#3](https://github.com/Myraxion/dion/issues/3) | TC 多行与程序扩展读取 | [#2](https://github.com/Myraxion/dion/issues/2) |
| [#4](https://github.com/Myraxion/dion/issues/4) | 列出目录备注 | [#3](https://github.com/Myraxion/dion/issues/3) |
| [#5](https://github.com/Myraxion/dion/issues/5) | 单行设置与 Windows 文件提交 | [#3](https://github.com/Myraxion/dion/issues/3) |
| [#6](https://github.com/Myraxion/dion/issues/6) | 多行设置与三种输入来源 | [#5](https://github.com/Myraxion/dion/issues/5) |
| [#7](https://github.com/Myraxion/dion/issues/7) | 删除备注与最后记录清理 | [#5](https://github.com/Myraxion/dion/issues/5) |
| [#8](https://github.com/Myraxion/dion/issues/8) | Windows 长路径、根与链接定位 | [#4](https://github.com/Myraxion/dion/issues/4)、[#7](https://github.com/Myraxion/dion/issues/7) |
| [#9](https://github.com/Myraxion/dion/issues/9) | UNC 网络共享读写 | [#6](https://github.com/Myraxion/dion/issues/6)、[#8](https://github.com/Myraxion/dion/issues/8) |
| [#10](https://github.com/Myraxion/dion/issues/10) | Total Commander 双向互操作 | [#4](https://github.com/Myraxion/dion/issues/4)、[#6](https://github.com/Myraxion/dion/issues/6)、[#7](https://github.com/Myraxion/dion/issues/7) |
| [#11](https://github.com/Myraxion/dion/issues/11) | Release 单 exe 与性能基线 | [#9](https://github.com/Myraxion/dion/issues/9)、[#10](https://github.com/Myraxion/dion/issues/10) |

发布时可以立即开始的是 Issue #2。后续从所有阻塞项已完成的任务中选取；执行时以 GitHub 当前原生依赖状态为准。各 Issue 均包含交付行为、验收标准和父规格引用，完整任务正文与标签已回读核验。

任务发布过程中未修改父规格的正文、标题、状态、标签或评论。此索引记录任务拆分，不表示已开始或完成实现。
