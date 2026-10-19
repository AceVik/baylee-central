param([switch]$ExerciseFailure)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Split-Path $PSScriptRoot)
$process = $null
$processStarted = $false
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
    $processStarted = $process.Start()
    if (-not $processStarted) { throw 'Cannot start owned development process' }
    $stderr = $process.StandardError.ReadToEndAsync()
    $lineTask = $process.StandardOutput.ReadLineAsync()
    if (-not $lineTask.Wait(15000)) { throw 'Development startup timed out' }
    $line = $lineTask.Result
    if ($line -notmatch '^central: development listener at (http://127\.0\.0\.1:\d+)$') {
        throw 'Unexpected development startup result'
    }
    $address = [Uri]$Matches[1]
    if ($ExerciseFailure) { throw 'Synthetic development failure' }
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
            # Accept only an explicit peer reset/abort. Other I/O failures are not
            # evidence that the server enforced its header deadline.
            $cause = $_.Exception.GetBaseException()
            if ($cause -isnot [Net.Sockets.SocketException] -or
                $cause.SocketErrorCode -notin @([Net.Sockets.SocketError]::ConnectionReset, [Net.Sockets.SocketError]::ConnectionAborted)) { throw }
            $read = -1
        }
        if ($read -ne -1 -or $watch.Elapsed.TotalSeconds -gt 4.5 -or $watch.Elapsed.TotalSeconds -lt 2) { throw 'Header deadline did not close stalled connection' }
    } finally { $socket.Dispose() }
} finally {
    # One failed cleanup operation must not skip the remaining owned resources.
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    try { if ($null -ne $client) { $client.Dispose() } }
    catch { $cleanupErrors.Add('Owned HTTP client cleanup failed') }
    try {
        if ($null -ne $process) {
            try {
                if ($processStarted) {
                    if (-not $process.HasExited) { $process.Kill($true) }
                    if (-not $process.WaitForExit(10000)) { throw 'process remains' }
                }
            } finally { $process.Dispose() }
        }
    } catch { $cleanupErrors.Add('Owned development process cleanup could not be verified') }
    try {
        if ($null -ne $address) {
            $probe = [Net.Sockets.TcpClient]::new()
            try {
                try {
                    $task = $probe.ConnectAsync($address.Host, $address.Port)
                    if (-not $task.Wait(5000)) { throw 'listener probe timed out' }
                    throw 'listener remains'
                } catch [AggregateException] {
                    $cause = $_.Exception.GetBaseException()
                    if ($cause -isnot [Net.Sockets.SocketException] -or
                        $cause.SocketErrorCode -ne [Net.Sockets.SocketError]::ConnectionRefused) { throw }
                }
            } finally { $probe.Dispose() }
        }
    } catch { $cleanupErrors.Add('Owned development listener closure could not be verified') }
    Pop-Location
    if ($cleanupErrors.Count -gt 0) { throw ($cleanupErrors -join '; ') }
}
Write-Output 'Development smoke passed: HTTP boundaries, header deadline and verified process/listener cleanup.'
