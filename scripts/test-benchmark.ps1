# Run with PowerShell 7 after cargo build --release --locked.
$ErrorActionPreference = 'Stop'
$benchmarkPath = Join-Path $PSScriptRoot 'benchmark.ps1'
$ast = [System.Management.Automation.Language.Parser]::ParseFile($benchmarkPath, [ref]$null, [ref]$null)
# Load the script's real measurement code without generating benchmark datasets.
$nativeCode = $ast.Find({ param($node)
    $node -is [System.Management.Automation.Language.CommandAst] -and $node.GetCommandName() -eq 'Add-Type'
}, $true)
Invoke-Expression $nativeCode.Extent.Text
$sizeVerdict = $ast.Find({ param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-SizeVerdict'
}, $true)
Invoke-Expression $sizeVerdict.Extent.Text

$exe = (Resolve-Path (Join-Path $PSScriptRoot '../target/release/dion.exe')).Path
$testDirectory = Join-Path ([IO.Path]::GetTempPath()) ('dion_metrics_test_' + [Guid]::NewGuid())
$null = New-Item -ItemType Directory -Path $testDirectory
try {
for ($run = 0; $run -lt 10; $run++) {
    $start = [Diagnostics.ProcessStartInfo]::new($exe)
    $start.ArgumentList.Add('list')
    $start.WorkingDirectory = $testDirectory
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        $handle = $process.Handle
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $null = $stdout.Result
        if ($process.ExitCode -ne 0) { throw $stderr.Result }
        # The old Job measurement returned zero when attached after this exit.
        if ([BenchmarkMetrics]::GetPeakMemory($handle) -le 0) { throw 'Missing lifetime peak' }
        [double]$lifetime = 0
        [double]$cpu = 0
        [BenchmarkMetrics]::GetTimes($handle, [ref]$lifetime, [ref]$cpu)
        if ($lifetime -le 0 -or $cpu -lt 0) { throw 'Invalid process times' }
    } finally {
        $process.Dispose()
    }
}
} finally {
    # list does not create files; remove only the known empty test directory.
    Remove-Item -LiteralPath $testDirectory
}

$failed = $false
try { $null = [BenchmarkMetrics]::GetPeakMemory([IntPtr]::Zero) } catch { $failed = $true }
if (-not $failed) { throw 'Invalid memory query was accepted' }
$failed = $false
try { [BenchmarkMetrics]::GetTimes([IntPtr]::Zero, [ref]$lifetime, [ref]$cpu) } catch { $failed = $true }
if (-not $failed) { throw 'Invalid time query was accepted' }
if ((Get-SizeVerdict 5MB) -ne '达成' -or (Get-SizeVerdict (5MB + 1)) -ne '未达成') {
    throw 'Incorrect size threshold verdict'
}
Write-Host 'PASS: exited-process peaks (10 runs), query failures, and 5 MiB size boundary'
