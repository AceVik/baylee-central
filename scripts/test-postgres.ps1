param([switch]$ExerciseFailure)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Split-Path $PSScriptRoot)
# Official multi-architecture PostgreSQL 18 image, pinned rather than a moving tag.
$image = 'postgres@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873'
$name = 'central-test-' + [IO.Path]::GetRandomFileName().Replace('.', '')
$containerId = $null
$testProcess = $null
$port = $null
$adminSecret = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
$migratorSecret = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
$runtimeSecret = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
$previousPassword = [Environment]::GetEnvironmentVariable('POSTGRES_PASSWORD')
try {
    Write-Output "Owned PostgreSQL fixture: $name"
    & docker image inspect $image *> $null
    if ($LASTEXITCODE -ne 0) {
        & docker pull $image
        if ($LASTEXITCODE -ne 0) { throw 'Pinned PostgreSQL image unavailable' }
    }
    # Only this child shell gets the synthetic admin credential; no value in argv.
    [Environment]::SetEnvironmentVariable('POSTGRES_PASSWORD', $adminSecret)
    $created = & docker create --name $name --label baylee.central.test=owned --memory 512m --cpus 2 --pids-limit 128 --read-only --tmpfs /var/lib/postgresql:rw,nosuid,nodev,size=268435456 --tmpfs /var/run/postgresql:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev --publish 127.0.0.1::5432 --env POSTGRES_PASSWORD --env PGDATA=/var/lib/postgresql/test $image 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'Creating isolated PostgreSQL fixture failed' }
    [Environment]::SetEnvironmentVariable('POSTGRES_PASSWORD', $previousPassword)
    $candidateId = ($created | Out-String).Trim()
    if ($candidateId -notmatch '^[0-9a-f]{64}$') { throw 'Invalid owned container identity' }
    $containerId = $candidateId
    & docker start $containerId | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Starting PostgreSQL fixture failed' }
    $ready = $false
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        & docker exec $containerId pg_isready -q -h 127.0.0.1 -U postgres *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) { throw 'PostgreSQL fixture did not become ready' }
    $portText = (& docker port $containerId 5432/tcp | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $portText -notmatch '^127\.0\.0\.1:(\d+)$') {
        throw 'Fixture must publish exclusively on IPv4 loopback'
    }
    $port = [int]$Matches[1]
    if ($ExerciseFailure) { throw 'Synthetic fixture failure' }
    # Secrets are generated hex, never caller-provided SQL; output is withheld on errors.
    $bootstrap = @"
CREATE ROLE central_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '$migratorSecret';
CREATE ROLE central_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '$runtimeSecret';
CREATE ROLE central_inherited LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '$runtimeSecret';
GRANT central_migrator TO central_inherited;
CREATE DATABASE central_test OWNER central_migrator;
\connect central_test
REVOKE CREATE, TEMP ON DATABASE central_test FROM PUBLIC;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
"@
    $bootstrapOutput = $bootstrap | & docker exec -i $containerId psql -X -v ON_ERROR_STOP=1 -U postgres -d postgres 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'Fixture role bootstrap failed (diagnostics withheld)' }
    $version = (& docker exec $containerId psql -X -At -U postgres -c 'SHOW server_version' | Out-String).Trim()
    Write-Output "Testing real PostgreSQL $version with isolated migration/runtime roles."
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = 'cargo'
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    [void]$start.Environment.Remove('POSTGRES_PASSWORD')
    [void]$start.Environment.Remove('PGPASSWORD')
    [void]$start.Environment.Remove('DATABASE_URL')
    foreach ($argument in @('test', '--locked', '-p', 'central-postgres', '--test', 'postgres', '--', '--ignored', '--test-threads=1')) {
        $start.ArgumentList.Add($argument)
    }
    $start.Environment['CENTRAL_TEST_FIXTURE'] = 'isolated-v1'
    $start.Environment['CENTRAL_TEST_PORT'] = "$port"
    $start.Environment['CENTRAL_TEST_MIGRATOR_PASSWORD'] = $migratorSecret
    $start.Environment['CENTRAL_TEST_RUNTIME_PASSWORD'] = $runtimeSecret
    $testProcess = [Diagnostics.Process]::Start($start)
    $testOutput = $testProcess.StandardOutput.ReadToEndAsync()
    $testErrors = $testProcess.StandardError.ReadToEndAsync()
    if (-not $testProcess.WaitForExit(300000)) { throw 'PostgreSQL integration tests timed out' }
    $resultText = $testOutput.GetAwaiter().GetResult()
    $errorText = $testErrors.GetAwaiter().GetResult()
    Write-Output $resultText
    if ($testProcess.ExitCode -ne 0) { Write-Output $errorText; throw 'PostgreSQL integration tests failed' }
    if ($resultText -notmatch 'test result: ok\. 1 passed; 0 failed') { throw 'Expected database test did not run' }
} finally {
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    try {
        if ($null -ne $testProcess) {
            try {
                if (-not $testProcess.HasExited) { $testProcess.Kill($true) }
                if (-not $testProcess.WaitForExit(10000)) { throw 'process remains' }
            } finally { $testProcess.Dispose() }
        }
    } catch { $cleanupErrors.Add('Owned integration process cleanup could not be verified') }
    try { [Environment]::SetEnvironmentVariable('POSTGRES_PASSWORD', $previousPassword) }
    catch { $cleanupErrors.Add('Temporary fixture environment could not be restored') }
    try {
        # The preallocated random name also handles an interruption during create.
        $found = & docker ps -a --no-trunc --filter "name=^/$name$" --format '{{.ID}} {{.Label "baylee.central.test"}}' 2>$null
        if ($LASTEXITCODE -ne 0) { throw 'container inventory unavailable' }
        $foundText = ($found | Out-String).Trim()
        if ($foundText.Length -gt 0) {
            if ($foundText -notmatch '^([0-9a-f]{64}) owned$') { throw 'ownership marker mismatch' }
            $ownedId = $Matches[1]
            if ($null -ne $containerId -and $ownedId -ne $containerId) { throw 'container identity changed' }
            & docker rm --force --volumes $ownedId | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'container removal failed' }
            $remaining = & docker ps -a --no-trunc --filter "id=$ownedId" --format '{{.ID}}' 2>$null
            if ($LASTEXITCODE -ne 0 -or ($remaining | Out-String).Trim().Length -gt 0) {
                throw 'container removal unverified'
            }
        }
    } catch { $cleanupErrors.Add('Owned PostgreSQL container cleanup could not be verified') }
    try {
        if ($null -ne $port) {
            $socket = [Net.Sockets.TcpClient]::new()
            try {
                try {
                    $task = $socket.ConnectAsync('127.0.0.1', $port)
                    if (-not $task.Wait(5000)) { throw 'listener probe timed out' }
                    throw 'listener remains'
                } catch [AggregateException] {
                    $cause = $_.Exception.GetBaseException()
                    if ($cause -isnot [Net.Sockets.SocketException] -or
                        $cause.SocketErrorCode -ne [Net.Sockets.SocketError]::ConnectionRefused) { throw }
                }
            } finally { $socket.Dispose() }
        }
    } catch { $cleanupErrors.Add('Owned PostgreSQL listener closure could not be verified') }
    Pop-Location
    if ($cleanupErrors.Count -gt 0) { throw ($cleanupErrors -join '; ') }
}
Write-Output 'PostgreSQL fixture passed and its owned container/listener were removed.'
exit 0
