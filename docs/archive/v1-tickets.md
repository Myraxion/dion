# Dion 第一版任务拆分（归档）

本文件于 2026-10-08 归档。第一版任务已经结束，其中 #9 网络共享按用户决定取消，其余任务完成。下文保留发布时的顺序、依赖和交付记录，不表示当前待办；后续任务状态以 GitHub Issues 为准。

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
| [#9](https://github.com/Myraxion/dion/issues/9) | 已取消：UNC 网络共享读写 | 不再作为第一版任务或发布阻塞项 |
| [#10](https://github.com/Myraxion/dion/issues/10) | Total Commander 双向互操作 | [#4](https://github.com/Myraxion/dion/issues/4)、[#6](https://github.com/Myraxion/dion/issues/6)、[#7](https://github.com/Myraxion/dion/issues/7) |
| [#11](https://github.com/Myraxion/dion/issues/11) | Release 单 exe 与本地性能基线 | [#10](https://github.com/Myraxion/dion/issues/10) |

发布时可以立即开始的是 Issue #2。后续从所有阻塞项已完成的任务中选取；执行时以 GitHub 当前原生依赖状态为准。各 Issue 均包含交付行为、验收标准和父规格引用，完整任务正文与标签已回读核验。

任务发布过程中未修改父规格的正文、标题、状态、标签或评论。此索引记录任务拆分，不表示已开始或完成实现。

2026-10-08 用户取消网络共享支持承诺、专用共享测试和验收入口；#9 按取消范围关闭，#11 移除对 #9 的原生阻塞依赖，并仅要求本地性能基线。#1 和 #8 的当前要求同步限定本地范围；此前实测结果归入[历史记录](unc-validation.md)。
2026-10-08 完成 Issue #10 Total Commander 双向互操作实测与黑盒测试套件，详细验收结果见 [TC 互操作实测报告](../tc-interop.md)。
2026-10-08 完成 Issue #11 Release 单 exe 构建与体积核验（354,816 字节，约 346.5 KiB，达成且远低于 5 MiB 目标）、全量黑盒测试通过及 100/10,000 条记录本地性能基线测量，详见 [本地性能基线报告](../benchmarks.md)。第一版规划任务全部交付完成。
