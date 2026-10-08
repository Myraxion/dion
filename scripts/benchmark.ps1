param(
    [string]$ExePath = "$PSScriptRoot\..\target\release\dion.exe",
    [ValidateRange(1, 10000)]
    [int]$Iterations = 5,
    [string]$OutputDir = "$PSScriptRoot\..\docs",
    [string]$StorageDescription = '未人工核验；见下方临时数据目录'
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @"
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class BenchmarkMetrics {
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool K32GetProcessMemoryInfo(IntPtr process, out PROCESS_MEMORY_COUNTERS counters, uint size);

    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool GetProcessTimes(IntPtr hProcess, out long lpCreationTime, out long lpExitTime, out long lpKernelTime, out long lpUserTime);

    [StructLayout(LayoutKind.Sequential)]
    struct PROCESS_MEMORY_COUNTERS {
        public uint cb;
        public uint PageFaultCount;
        public UIntPtr PeakWorkingSetSize;
        public UIntPtr WorkingSetSize;
        public UIntPtr QuotaPeakPagedPoolUsage;
        public UIntPtr QuotaPagedPoolUsage;
        public UIntPtr QuotaPeakNonPagedPoolUsage;
        public UIntPtr QuotaNonPagedPoolUsage;
        public UIntPtr PagefileUsage;
        public UIntPtr PeakPagefileUsage;
    }

    // Query the kernel's lifetime peak through the retained handle after exit.
    // PeakPagefileUsage is commit charge, not the resident working set.
    public static ulong GetPeakMemory(IntPtr process) {
        PROCESS_MEMORY_COUNTERS counters;
        uint size = (uint)Marshal.SizeOf(typeof(PROCESS_MEMORY_COUNTERS));
        if (!K32GetProcessMemoryInfo(process, out counters, size)) {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Peak memory query failed");
        }
        ulong peak = counters.PeakPagefileUsage.ToUInt64();
        if (peak == 0) throw new InvalidOperationException("Peak memory measurement is zero");
        return peak;
    }

    public static void GetTimes(IntPtr process, out double processLifetimeMs, out double cpuTimeMs) {
        long creationTime, exitTime, kernelTime, userTime;
        if (!GetProcessTimes(process, out creationTime, out exitTime, out kernelTime, out userTime)) {
            throw new Win32Exception(Marshal.GetLastWin32Error(), "Process time query failed");
        }
        processLifetimeMs = (exitTime - creationTime) / 10000.0;
        cpuTimeMs = (kernelTime + userTime) / 10000.0;
    }
}
"@

$ResolvedExe = (Resolve-Path $ExePath).Path
if (-not (Test-Path $ResolvedExe)) {
    throw "Release 可执行文件不存在: $ResolvedExe。请先运行 cargo build --release --locked。"
}

$ExeFileInfo = Get-Item $ResolvedExe
$ExeSizeBytes = $ExeFileInfo.Length
$ExeSizeMiB = [math]::Round($ExeSizeBytes / 1MB, 2)
$ExeSizeKiB = [math]::Round($ExeSizeBytes / 1KB, 1)

Write-Host "=== Dion Release 性能基准测试 ===" -ForegroundColor Cyan
Write-Host "Exe 路径: $ResolvedExe"
Write-Host "Exe 体积: $ExeSizeBytes 字节 ($ExeSizeKiB KiB, $ExeSizeMiB MiB)"
Write-Host "测试迭代轮数: $Iterations"

$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$machineInfo = [PSCustomObject]@{
    OSCaption     = $os.Caption
    OSVersion     = $os.Version
    OSArch        = $os.OSArchitecture
    CPU           = $cpu.Name
    Cores         = $cpu.NumberOfCores
    LogicalCPUs   = $cpu.NumberOfLogicalProcessors
    TotalRAM_GB   = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
}

function Invoke-MeasuredRun {
    param(
        [string]$Exe,
        [string[]]$Arguments,
        [string]$WorkingDir
    )

    $p = $null
    try {
        $psi = [System.Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = $Exe
        foreach ($a in $Arguments) {
            $psi.ArgumentList.Add($a)
        }
        $psi.WorkingDirectory = $WorkingDir
        $psi.UseShellExecute = $false
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.CreateNoWindow = $true

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $p = [System.Diagnostics.Process]::Start($psi)
        $handle = $p.Handle
        $outTask = $p.StandardOutput.ReadToEndAsync()
        $errTask = $p.StandardError.ReadToEndAsync()
        $p.WaitForExit()
        $sw.Stop()

        $out = $outTask.Result
        $err = $errTask.Result
        $peakMem = [BenchmarkMetrics]::GetPeakMemory($handle)
        [double]$procLife = 0; [double]$cpuTime = 0
        [BenchmarkMetrics]::GetTimes($handle, [ref]$procLife, [ref]$cpuTime)

        if ($p.ExitCode -ne 0) {
            throw "命令执行失败 (ExitCode $($p.ExitCode)): $err"
        }

        return [PSCustomObject]@{
            WallClockMs       = $sw.Elapsed.TotalMilliseconds
            ProcessLifetimeMs = $procLife
            CpuTimeMs         = $cpuTime
            PeakMemBytes      = $peakMem
        }
    } finally {
        if ($null -ne $p) { $p.Dispose() }
    }
}

function Get-SizeVerdict([long]$Bytes) {
    if ($Bytes -le 5MB) { '达成' } else { '未达成' }
}

function Format-MeasurementRange($Rows, [string]$Property) {
    $range = $Rows | Measure-Object -Property $Property -Minimum -Maximum
    '{0:F2}–{1:F2}' -f $range.Minimum, $range.Maximum
}

$benchRoot = Join-Path ([System.IO.Path]::GetTempPath()) "dion_benchmarks_$(Get-Random)"
New-Item -ItemType Directory -Path $benchRoot | Out-Null

try {
    $emptyDir = Join-Path $benchRoot "empty_dir"
    New-Item -ItemType Directory -Path $emptyDir | Out-Null

    function Generate-Dataset {
        param(
            [string]$TargetDir,
            [int]$Count
        )

        New-Item -ItemType Directory -Path $TargetDir | Out-Null
        $descPath = Join-Path $TargetDir "descript.ion"

        $fs = [System.IO.File]::Create($descPath)
        $bom = [byte[]](0xEF, 0xBB, 0xBF)
        $fs.Write($bom, 0, 3)

        $writer = [System.IO.StreamWriter]::new($fs, [System.Text.UTF8Encoding]::new($false))
        # TC UTF-8 扩展标记 0x04 0xC3 0x82 对应 Unicode 字符 \u0004\u00C2
        $tcExt = [char]0x0004 + [char]0x00C2

        for ($i = 1; $i -le $Count; $i++) {
            $num = "{0:D5}" -f $i
            $filename = "item_$num.txt"
            $filePath = Join-Path $TargetDir $filename
            [System.IO.File]::WriteAllBytes($filePath, [byte[]]@())

            if ($i % 5 -eq 0) {
                # 多行记录带 TC 扩展标记
                $writer.Write("`"$filename`" 第一行备注\n第二行包含反斜杠\\符号$tcExt`r`n")
            } elseif ($i % 3 -eq 0) {
                # 引号包围单行带 Unicode 表情符号
                $writer.Write("`"$filename`" 普通单行备注编号 $num 包含 Unicode 符号 😀`r`n")
            } else {
                # 普通单行
                $writer.Write("$filename 备注编号 $num`r`n")
            }
        }
        $writer.Flush()
        $writer.Close()
        $fs.Close()

        $backupDesc = Join-Path $TargetDir "descript.ion.bak"
        Copy-Item -Path $descPath -Destination $backupDesc
    }

    function Reset-Dataset {
        param(
            [string]$TargetDir,
            [string]$EnsureItemName
        )
        $backup = Join-Path $TargetDir "descript.ion.bak"
        $target = Join-Path $TargetDir "descript.ion"
        Copy-Item -Path $backup -Destination $target -Force
        if (-not [string]::IsNullOrEmpty($EnsureItemName)) {
            $itemPath = Join-Path $TargetDir $EnsureItemName
            if (-not (Test-Path $itemPath)) {
                [System.IO.File]::WriteAllBytes($itemPath, [byte[]]@())
            }
        }
    }

    Write-Host "正在生成 100 条记录数据集..." -ForegroundColor Yellow
    $dir100 = Join-Path $benchRoot "dataset_100"
    Generate-Dataset -TargetDir $dir100 -Count 100

    Write-Host "正在生成 10,000 条记录数据集..." -ForegroundColor Yellow
    $dir10k = Join-Path $benchRoot "dataset_10000"
    Generate-Dataset -TargetDir $dir10k -Count 10000
    $size100 = (Get-Item (Join-Path $dir100 'descript.ion')).Length
    $size10k = (Get-Item (Join-Path $dir10k 'descript.ion')).Length

    $testScenarios = @(
        # 最简空目录冷启动
        @{ Name = "Cold start (empty dir list)"; Dir = $emptyDir; Args = @("list"); Target = "empty"; IsCold = $true },

        # 100 条记录场景：冷启动覆盖 get / list / set / remove
        @{ Name = "100 records: cold get"; Dir = $dir100; Args = @("get", "item_00050.txt"); Target = "100"; IsCold = $true },
        @{ Name = "100 records: cold list"; Dir = $dir100; Args = @("list"); Target = "100"; IsCold = $true },
        @{ Name = "100 records: cold set"; Dir = $dir100; Args = @("set", "item_00050.txt", "更新的备注内容"); Target = "100"; IsCold = $true; Mutates = $true; EnsureItem = "item_00050.txt" },
        @{ Name = "100 records: cold remove"; Dir = $dir100; Args = @("remove", "item_00050.txt"); Target = "100"; IsCold = $true; Mutates = $true; EnsureItem = "item_00050.txt" },

        # 100 条记录温运行测量
        @{ Name = "100 records: warm get (text)"; Dir = $dir100; Args = @("get", "item_00050.txt"); Target = "100"; IsCold = $false },
        @{ Name = "100 records: warm get (json)"; Dir = $dir100; Args = @("get", "item_00050.txt", "--json"); Target = "100"; IsCold = $false },
        @{ Name = "100 records: warm list (text)"; Dir = $dir100; Args = @("list"); Target = "100"; IsCold = $false },
        @{ Name = "100 records: warm list (json)"; Dir = $dir100; Args = @("list", "--json"); Target = "100"; IsCold = $false },
        @{ Name = "100 records: warm set (update)"; Dir = $dir100; Args = @("set", "item_00050.txt", "温运行更新内容"); Target = "100"; IsCold = $false; Mutates = $true; EnsureItem = "item_00050.txt" },
        @{ Name = "100 records: warm remove"; Dir = $dir100; Args = @("remove", "item_00050.txt"); Target = "100"; IsCold = $false; Mutates = $true; EnsureItem = "item_00050.txt" },

        # 10,000 条记录场景：冷启动覆盖 get / list / set / remove
        @{ Name = "10,000 records: cold get"; Dir = $dir10k; Args = @("get", "item_05000.txt"); Target = "10000"; IsCold = $true },
        @{ Name = "10,000 records: cold list"; Dir = $dir10k; Args = @("list"); Target = "10000"; IsCold = $true },
        @{ Name = "10,000 records: cold set"; Dir = $dir10k; Args = @("set", "item_05000.txt", "更新的备注内容"); Target = "10000"; IsCold = $true; Mutates = $true; EnsureItem = "item_05000.txt" },
        @{ Name = "10,000 records: cold remove"; Dir = $dir10k; Args = @("remove", "item_05000.txt"); Target = "10000"; IsCold = $true; Mutates = $true; EnsureItem = "item_05000.txt" },

        # 10,000 条记录温运行测量
        @{ Name = "10,000 records: warm get (text)"; Dir = $dir10k; Args = @("get", "item_05000.txt"); Target = "10000"; IsCold = $false },
        @{ Name = "10,000 records: warm get (json)"; Dir = $dir10k; Args = @("get", "item_05000.txt", "--json"); Target = "10000"; IsCold = $false },
        @{ Name = "10,000 records: warm list (text)"; Dir = $dir10k; Args = @("list"); Target = "10000"; IsCold = $false },
        @{ Name = "10,000 records: warm list (json)"; Dir = $dir10k; Args = @("list", "--json"); Target = "10000"; IsCold = $false },
        @{ Name = "10,000 records: warm set (update)"; Dir = $dir10k; Args = @("set", "item_05000.txt", "温运行更新内容"); Target = "10000"; IsCold = $false; Mutates = $true; EnsureItem = "item_05000.txt" },
        @{ Name = "10,000 records: warm remove"; Dir = $dir10k; Args = @("remove", "item_05000.txt"); Target = "10000"; IsCold = $false; Mutates = $true; EnsureItem = "item_05000.txt" }
    )

    $results = @()

    foreach ($scenario in $testScenarios) {
        $sName = $scenario.Name
        Write-Host "正在测量: $sName ..." -NoNewline

        if ($scenario.Mutates) {
            Reset-Dataset -TargetDir $scenario.Dir -EnsureItemName $scenario.EnsureItem
        }

        if ($scenario.IsCold) {
            $coldRes = Invoke-MeasuredRun -Exe $ResolvedExe -Arguments $scenario.Args -WorkingDir $scenario.Dir
            if ($scenario.Mutates) {
                Reset-Dataset -TargetDir $scenario.Dir -EnsureItemName $scenario.EnsureItem
            }
            $results += [PSCustomObject]@{
                Scenario      = $sName
                Target        = $scenario.Target
                Iterations    = 1
                WallClockMean = [math]::Round($coldRes.WallClockMs, 2)
                WallClockMed  = [math]::Round($coldRes.WallClockMs, 2)
                CpuTimeMean   = [math]::Round($coldRes.CpuTimeMs, 2)
                ProcLifeMean  = [math]::Round($coldRes.ProcessLifetimeMs, 2)
                PeakMemMB     = [math]::Round($coldRes.PeakMemBytes / 1MB, 2)
            }
            Write-Host " 完成 (壁钟: $($coldRes.WallClockMs) ms, CPU: $($coldRes.CpuTimeMs) ms, 内存: $([math]::Round($coldRes.PeakMemBytes / 1MB, 2)) MB)" -ForegroundColor Green
        } else {
            $wallTimes = @()
            $cpuTimes = @()
            $procTimes = @()
            $mems = @()

            # 预热一次
            Invoke-MeasuredRun -Exe $ResolvedExe -Arguments $scenario.Args -WorkingDir $scenario.Dir | Out-Null
            if ($scenario.Mutates) {
                Reset-Dataset -TargetDir $scenario.Dir -EnsureItemName $scenario.EnsureItem
            }

            for ($iter = 1; $iter -le $Iterations; $iter++) {
                $res = Invoke-MeasuredRun -Exe $ResolvedExe -Arguments $scenario.Args -WorkingDir $scenario.Dir
                $wallTimes += $res.WallClockMs
                $cpuTimes  += $res.CpuTimeMs
                $procTimes += $res.ProcessLifetimeMs
                $mems      += $res.PeakMemBytes
                if ($scenario.Mutates) {
                    Reset-Dataset -TargetDir $scenario.Dir -EnsureItemName $scenario.EnsureItem
                }
            }

            $sortedWall = $wallTimes | Sort-Object
            $middle = [math]::Floor($wallTimes.Count / 2)
            $medWall = if ($wallTimes.Count % 2) { $sortedWall[$middle] } else {
                ($sortedWall[$middle - 1] + $sortedWall[$middle]) / 2
            }
            $avgWall = ($wallTimes | Measure-Object -Average).Average
            $avgCpu = ($cpuTimes | Measure-Object -Average).Average
            $avgProc = ($procTimes | Measure-Object -Average).Average
            $peakMemMax = ($mems | Measure-Object -Maximum).Maximum

            $results += [PSCustomObject]@{
                Scenario      = $sName
                Target        = $scenario.Target
                Iterations    = $Iterations
                WallClockMean = [math]::Round($avgWall, 2)
                WallClockMed  = [math]::Round($medWall, 2)
                CpuTimeMean   = [math]::Round($avgCpu, 2)
                ProcLifeMean  = [math]::Round($avgProc, 2)
                PeakMemMB     = [math]::Round($peakMemMax / 1MB, 2)
            }
            Write-Host " 完成 (壁钟均值: $([math]::Round($avgWall, 2)) ms, CPU均值: $([math]::Round($avgCpu, 2)) ms, 峰值内存: $([math]::Round($peakMemMax / 1MB, 2)) MB)" -ForegroundColor Green
        }
    }

    Write-Host "`n=== 测量结果汇总 ===" -ForegroundColor Cyan
    $results | Format-Table Scenario, WallClockMean, WallClockMed, CpuTimeMean, ProcLifeMean, PeakMemMB -AutoSize

    $dateStr = [TimeZoneInfo]::ConvertTimeBySystemTimeZoneId([DateTime]::UtcNow, 'China Standard Time').ToString('yyyy-MM-dd HH:mm:ss')
    $sizeVerdict = Get-SizeVerdict $ExeSizeBytes
    $warm100 = $results | Where-Object { $_.Target -eq '100' -and $_.Scenario -like '*warm*' }
    $warm10k = $results | Where-Object { $_.Target -eq '10000' -and $_.Scenario -like '*warm*' }
    $md = @"
# Dion 本地性能基线报告 (Issue #11)

- 测量日期: $dateStr (Asia/Shanghai)
- 可执行文件: ``$($ExeFileInfo.Name)`` (Release 构建, ``cargo build --release --locked``)
- 产物体积: **$ExeSizeBytes 字节** ($ExeSizeKiB KiB, **$ExeSizeMiB MiB**)
  - 目标上限: 5.00 MiB
  - 达成情况: $sizeVerdict (占目标上限约 $([math]::Round(($ExeSizeBytes / (5 * 1024 * 1024)) * 100, 1))%)
- 复现: 在 PowerShell 7 执行 ``cargo build --release --locked``，然后执行 ``./scripts/benchmark.ps1``；可用 ``-StorageDescription`` 记录人工核验的存储环境。
- 构建配置: 默认 Release 产物由仓库配置静态链接 MSVC CRT；脚本不检查自定义 ``-ExePath`` 的构建模式或依赖。

## 测量机器环境与测试条件

| 属性 | 规格 / 配置 |
| --- | --- |
| 操作系统 | $($machineInfo.OSCaption) (版本 $($machineInfo.OSVersion), $($machineInfo.OSArch)) |
| 处理器 (CPU) | $($machineInfo.CPU) ($($machineInfo.Cores) 核心 / $($machineInfo.LogicalCPUs) 逻辑处理器) |
| 物理内存 (RAM) | $($machineInfo.TotalRAM_GB) GB |
| 文件系统 / 存储 | $StorageDescription |
| 临时数据目录 | ``$benchRoot`` (运行结束后清理) |
| 测量范围 | 本地文件系统 (不包含网络共享/UNC/SMB) |
| 输出流处理 | 标准输出与标准错误重定向至后台管道并完全读取丢弃，不输出至终端，排除控制台渲染与字符滚动开销 |
| 峰值内存测量 | 保留进程句柄，退出后用 ``K32GetProcessMemoryInfo`` 读取 ``PROCESS_MEMORY_COUNTERS.PeakPagefileUsage``；生命周期峰值提交量，非工作集；查询失败或零值中止测量 |
| 时间测量维度 | 壁钟耗时 (Wall-Clock Time)、进程生命周期时间 (Process Lifetime via ``GetProcessTimes``) 与 CPU 占用时间 (CPU Time) |
| 预热与迭代 | 温运行场景执行 1 次预热，后续执行 $Iterations 轮独立测量取统计值；修改类操作每次运行前严格恢复基准数据集 |
| cold 场景口径 | 每个场景不额外预热的首次调用；不清空系统文件缓存，不代表磁盘或系统缓存冷启动 |

峰值提交量定义见 [Microsoft PROCESS_MEMORY_COUNTERS](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters)。

## 工作负载特征

- **100 条记录**:
  - 备注文件大小 $size100 字节。
  - 混合普通单行、引号包围的名称、含 Unicode 字符与 Emoji（😀）的正文以及 TC 多行转义扩展记录（含 ``\n``、``\\`` 及控制标记 ``04 C3 82``）。
- **10,000 条记录**:
  - 备注文件大小 $size10k 字节。
  - 同样混合普通单行、引号包围名称、Unicode 字符以及 TC 多行扩展转义记录。
- **关联文件条目**:
  - 目录下创建有所有对应的实际文件条目，满足 ``set`` 命令目标条目必须存在的前提约束。

## 本地性能基线数据

| 测量场景 | 迭代次数 | 平均壁钟耗时 (ms) | 中位数壁钟耗时 (ms) | 平均 CPU 耗时 (ms) | 平均进程生命周期 (ms) | 峰值提交量 (MiB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
"@

    foreach ($r in $results) {
        $md += "`n| $($r.Scenario) | $($r.Iterations) | $($r.WallClockMean) | $($r.WallClockMed) | $($r.CpuTimeMean) | $($r.ProcLifeMean) | $($r.PeakMemMB) |"
    }

    $md += @"


## 结果分析与时延门槛说明

- 本次 exe 体积为 $ExeSizeMiB MiB，5 MiB 目标：**$sizeVerdict**。
- 100 条记录温运行：各场景平均壁钟耗时 $(Format-MeasurementRange $warm100 'WallClockMean') ms，平均 CPU 耗时 $(Format-MeasurementRange $warm100 'CpuTimeMean') ms，峰值提交量 $(Format-MeasurementRange $warm100 'PeakMemMB') MiB。
- 10,000 条记录温运行：各场景平均壁钟耗时 $(Format-MeasurementRange $warm10k 'WallClockMean') ms，平均 CPU 耗时 $(Format-MeasurementRange $warm10k 'CpuTimeMean') ms，峰值提交量 $(Format-MeasurementRange $warm10k 'PeakMemMB') MiB。
- 上述区间为本次场景统计值的最小值与最大值，不是单次运行范围或性能保证；峰值内存结果不能证明不存在内存泄漏。
- 旧版报告使用 Job Object 峰值且混称工作集，本次统一为进程峰值提交量；不将两种口径的数值直接用于判断性能变化。
- 保留本地基线供后续确认时延预算，本次不新增固定时延门槛。

"@

    $reportPath = Join-Path $OutputDir "benchmarks.md"
    [System.IO.File]::WriteAllText($reportPath, $md, [System.Text.UTF8Encoding]::new($false))
    Write-Host "`n基线报告已成功写入: $reportPath" -ForegroundColor Green

} finally {
    if (Test-Path $benchRoot) {
        $cleanupPath = [IO.Path]::GetFullPath($benchRoot)
        $tempDirectory = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
        if (-not $cleanupPath.StartsWith($tempDirectory, [StringComparison]::OrdinalIgnoreCase) -or
            -not [IO.Path]::GetFileName($cleanupPath).StartsWith('dion_benchmarks_')) {
            throw "Unexpected benchmark cleanup path: $cleanupPath"
        }
        Remove-Item -LiteralPath $cleanupPath -Recurse -Force
    }
}
