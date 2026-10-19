$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Split-Path $PSScriptRoot)
$process = $null
$client = $null
$address = $null
try {
    $binary = Join-Path (Get-Location) 'target/release/central-service'
    if ($IsWindows) { $binary += '.exe' }
    if (-not (Test-Path -LiteralPath $binary)) { throw 'Build the release executable first' }
    $start = [Diagnostics.ProcessStartInfo]::new($binary)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.ArgumentList.Add('--dev')
    foreach ($key in @($start.Environment.Keys)) {
        if ($key.StartsWith('BAYLEE_CENTRAL_', [StringComparison]::OrdinalIgnoreCase)) {
            $start.Environment.Remove($key) | Out-Null
        }
    }
    $start.Environment['BAYLEE_CENTRAL_LISTEN'] = '127.0.0.1:0'
    $start.Environment['BAYLEE_CENTRAL_REQUEST_BYTES'] = '1024'
    $start.Environment['BAYLEE_CENTRAL_CONCURRENT_REQUESTS'] = '2'
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    if (-not $process.Start()) { throw 'Cannot start owned development process' }
    $stderr = $process.StandardError.ReadToEndAsync()
    $lineTask = $process.StandardOutput.ReadLineAsync()
    if (-not $lineTask.Wait(15000)) { throw 'Development startup timed out' }
    $line = $lineTask.Result
    if ($line -notmatch '^central: development listener at (http://127\.0\.0\.1:\d+)$') {
        throw 'Unexpected development startup result'
    }
    $address = [Uri]$Matches[1]
    $client = [Net.Http.HttpClient]::new()
    $client.Timeout = [TimeSpan]::FromSeconds(5)
    foreach ($case in @(
        @{Path='/health/live'; Status=200},
        @{Path='/health/ready'; Status=503},
        @{Path='/auth/login'; Status=404},
        @{Path='/v1/training'; Status=404}
    )) {
        $response = $client.GetAsync([Uri]::new($address, $case.Path)).GetAwaiter().GetResult()
        try {
            if ([int]$response.StatusCode -ne $case.Status) { throw 'Unexpected endpoint status' }
            if (-not $response.Headers.CacheControl.NoStore) { throw 'Missing no-store policy' }
        } finally { $response.Dispose() }
    }
    foreach ($case in @(
        @{Name='Origin'; Value='https://attacker.invalid'; Status=403},
        @{Name='Host'; Value='attacker.invalid'; Status=400}
    )) {
        $request = [Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::Get, [Uri]::new($address, '/health/live'))
        try {
            $request.Headers.TryAddWithoutValidation($case.Name, $case.Value) | Out-Null
            $response = $client.SendAsync($request).GetAwaiter().GetResult()
            try {
                if ([int]$response.StatusCode -ne $case.Status) { throw 'HTTP boundary did not reject attacker header' }
            } finally { $response.Dispose() }
        } finally { $request.Dispose() }
    }
    # Test the real pre-header timeout, outside request middleware.
    $socket = [Net.Sockets.TcpClient]::new()
    try {
        $socket.Connect($address.Host, $address.Port)
        $stream = $socket.GetStream()
        $stream.ReadTimeout = 5000
        $partial = [Text.Encoding]::ASCII.GetBytes("GET /health/live HTTP/1.1`r`nHost: ")
        $stream.Write($partial, 0, $partial.Length)
        $watch = [Diagnostics.Stopwatch]::StartNew()
        try {
            $reply = [Collections.Generic.List[byte]]::new()
            while (($read = $stream.ReadByte()) -ne -1) {
                $reply.Add([byte]$read)
                if ($reply.Count -gt 1024) { throw 'Unexpected timeout response size' }
            }
            if ($reply.Count -gt 0 -and [Text.Encoding]::ASCII.GetString($reply.ToArray()) -notmatch '^HTTP/1.1 408') {
                throw 'Stalled headers received an unexpected response'
            }
        } catch [IO.IOException] {
            # A reset is a valid close; a read timeout is not.
            if ($_.Exception.InnerException -is [Net.Sockets.SocketException] -and
                $_.Exception.InnerException.SocketErrorCode -eq [Net.Sockets.SocketError]::TimedOut) { throw }
            $read = -1
        }
        if ($read -ne -1 -or $watch.Elapsed.TotalSeconds -gt 4.5 -or $watch.Elapsed.TotalSeconds -lt 2) { throw 'Header deadline did not close stalled connection' }
    } finally { $socket.Dispose() }
    Write-Output 'Development smoke passed: real HTTP boundaries, readiness, and header deadline.'
} finally {
    if ($null -ne $client) { $client.Dispose() }
    if ($null -ne $process) {
        if (-not $process.HasExited) { $process.Kill() }
        if (-not $process.WaitForExit(10000)) { throw 'Owned process failed to exit' }
        $process.Dispose()
    }
    if ($null -ne $address) {
        $probe = [Net.Sockets.TcpClient]::new()
        try {
            $connected = $false
            try {
                $task = $probe.ConnectAsync($address.Host, $address.Port)
                if ($task.Wait(1000)) { $connected = $probe.Connected }
            } catch [AggregateException] { }
            if ($connected) { throw 'Development listener remained open after cleanup' }
        } finally { $probe.Dispose() }
    }
    Pop-Location
}
