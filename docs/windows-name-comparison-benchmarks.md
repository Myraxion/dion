# Windows 原生名称比较性能对比

2026-10-10，在同一 Windows 环境先测改动前产物，再测改动后产物。各温运行场景预热 1 次，测量 5 次；标准输出和错误重定向并完整读取。原有性能基线报告未被覆盖。

改动前源码为开始实施时的工作区快照 `c36c6b84a2061fe749ae839759b02cab94edfac6`，包括当时已有的 List 容错改动；改动后源码与采样时快照 `391875e4752d78683a422e47b17f1d85e5e01000` 一致。这两个快照保存在本地 Git 对象库，未作为发布提交；二进制 SHA-256 用于辨认本次实际被测产物。

## 环境与工作负载

| 属性 | 规格 / 配置 |
| --- | --- |
| 操作系统 | Microsoft Windows 11 IoT 企业版 LTSC (版本 10.0.26100, 64 位) |
| 处理器 (CPU) | AMD Ryzen 9 7940H w/ Radeon 780M Graphics      (8 核心 / 16 逻辑处理器) |
| 物理内存 (RAM) | 31.2 GB |
| 文件系统 / 存储 | 未人工核验；测试位于系统临时目录 |
| 测量范围 | 本地文件系统 (不包含网络共享/UNC/SMB) |
| 峰值内存测量 | 保留进程句柄，退出后用 `K32GetProcessMemoryInfo` 读取 `PROCESS_MEMORY_COUNTERS.PeakPagefileUsage`；生命周期峰值提交量，非工作集；查询失败或零值中止测量 |

100 条与 10,000 条记录分别为 5,330 与 531,350 字节，混合 Unicode 名称、普通备注和 TC 多行扩展；目录内创建所有对应条目。峰值为进程生命周期峰值提交量（PeakPagefileUsage），不是工作集。

## 产物

| 产物 | 字节 | KiB | SHA-256 |
| --- | ---: | ---: | --- |
| 改动前 | 515072 | 503.0 | `FEE92815AA5650AA6FE62EB8AE789E4E5AFCDBB0734EBAA3B8D61AB70D28C742` |
| 改动后 | 504832 | 493.0 | `04D9200400026EF4DE88AD1BCA3FB702286DC3577C5EC3211F2F2D3A3B36B962` |

## 温运行对比

壁钟及 CPU 均为毫秒均值，峰值提交量为 MiB。时延差为改动后减改动前。

| 场景 | 壁钟前 | 壁钟后 | 差值 | CPU 前 | CPU 后 | 峰值前 | 峰值后 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 records: warm get (text) | 183.91 | 183.53 | -0.38 | 140.62 | 134.38 | 9.36 | 9.38 |
| 100 records: warm get (json) | 175.03 | 159.95 | -15.08 | 131.25 | 103.12 | 9.37 | 9.36 |
| 100 records: warm list (text) | 168.52 | 179.7 | +11.18 | 115.62 | 121.88 | 9.4 | 9.41 |
| 100 records: warm list (json) | 174.32 | 170.89 | -3.43 | 118.75 | 118.75 | 9.39 | 9.46 |
| 100 records: warm set (update) | 191.34 | 185.29 | -6.05 | 134.38 | 131.25 | 9.39 | 9.39 |
| 100 records: warm remove | 182.83 | 172.82 | -10.01 | 125 | 128.12 | 9.39 | 9.42 |
| 10,000 records: warm get (text) | 181.79 | 185.95 | +4.16 | 125 | 137.5 | 11.19 | 11.42 |
| 10,000 records: warm get (json) | 179.56 | 182.06 | +2.50 | 131.25 | 137.5 | 11.5 | 11.36 |
| 10,000 records: warm list (text) | 223.68 | 230.37 | +6.69 | 171.88 | 181.25 | 12.88 | 13.06 |
| 10,000 records: warm list (json) | 219.5 | 223.59 | +4.09 | 156.25 | 171.88 | 19.22 | 19.35 |
| 10,000 records: warm set (update) | 186.74 | 186.88 | +0.14 | 128.12 | 121.88 | 11.42 | 11.65 |
| 10,000 records: warm remove | 189.1 | 196.86 | +7.76 | 128.12 | 153.12 | 11.13 | 11.62 |

## 解释与复现

10,000 条记录温运行壁钟变化约 +0.14 至 +7.76 ms；峰值提交量最大值由 19.22 增至 19.35 MiB，exe 减少 10 KiB，仍远低于 5 MiB 目标。100 条记录既有上升也有下降。本次未观察到与两两扫描相当的规模性退化；这些结果包含进程启动和调度噪声，不证明统计显著性或跨环境性能保证。

在 PowerShell 7 使用保存的改动前后 Release 产物，分别创建报告目录后运行：

```powershell
./scripts/benchmark.ps1 -ExePath <before.exe> -OutputDir <before-report-directory>
./scripts/benchmark.ps1 -ExePath <after.exe> -OutputDir <after-report-directory>
```

开发现场的被测产物和完整脚本输出报告分别保留在 `.scratch/native-names/{before,after}.exe` 与 `.scratch/native-names/{before,after}/benchmarks.md`；这些忽略文件不会随仓库提交分发。新版本重测应重新记录源码状态与二进制哈希。
