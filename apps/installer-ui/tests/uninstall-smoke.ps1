$ErrorActionPreference = 'Stop'
$testRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../../target/uninstall-verification'))
$install = Join-Path $testRoot '安装目录 with spaces'
$status = Join-Path $testRoot 'uninstall.status'
$bundle = 'red.ghs.axolotl.uninstall-fixture'
$roaming = Join-Path $env:APPDATA $bundle
$local = Join-Path $env:LOCALAPPDATA $bundle
$reg = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Axolotl Uninstall Fixture'
function Run-Core([string]$file, [string]$arguments) {
  $info = [System.Diagnostics.ProcessStartInfo]::new($file, $arguments)
  $info.UseShellExecute = $false
  $process = [System.Diagnostics.Process]::Start($info)
  if (!$process.WaitForExit(30000)) { throw 'Fixture timed out' }
  return $process.ExitCode
}
function Install-Fixture {
  $code = Run-Core (Join-Path $testRoot 'setup.exe') ('/S /NO_RUN_AFTER_INSTALL /INSTALL_DIR="' + $install + '"')
  if ($code -ne 0) { throw "Install failed: $code" }
  if (!(Test-Path (Join-Path $install 'AxolotlUninstallFixture.exe'))) { throw 'Missing main binary' }
  if (!(Test-Path $reg)) { throw 'Missing uninstall registration' }
  $script:shortcutPaths = @((Join-Path $env:APPDATA 'Microsoft/Windows/Start Menu/Programs/Axolotl Uninstall Fixture.lnk'), (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Axolotl Uninstall Fixture.lnk'))
  if (!($script:shortcutPaths | Where-Object { Test-Path $_ })) { throw 'Fixture did not create any shortcuts' }
  Copy-Item (Join-Path $install 'uninstall.exe') (Join-Path $testRoot 'core.exe') -Force
}
function Uninstall-Fixture([string]$options = '') {
  return Run-Core (Join-Path $testRoot 'core.exe') ('/S /UI_CHILD /STATUS_FILE="' + $status + '" ' + $options + ' _?=' + $install)
}
Install-Fixture
New-Item -ItemType Directory -Path $roaming,$local -Force | Out-Null
Set-Content (Join-Path $roaming 'keep.txt') 'config'
Set-Content (Join-Path $local 'keep.txt') 'cache'
Set-Content (Join-Path $install 'unrelated.txt') 'user-owned file'
$code = Uninstall-Fixture
if ($code -ne 0 -or (Get-Content $status -Raw) -ne '100') { throw "Uninstall failed: $code, $(Get-Content $status)" }
if (Test-Path (Join-Path $install 'AxolotlUninstallFixture.exe')) { throw 'Main binary remains' }
if (Test-Path (Join-Path $install 'resources\resource.txt')) { throw 'Resource remains' }
if (Test-Path (Join-Path $install 'uninstall.exe')) { throw 'Uninstaller remains' }
if (Test-Path $reg) { throw 'Registration remains' }
if ($shortcutPaths | Where-Object { Test-Path $_ }) { throw 'Shortcuts remain' }
if (!(Test-Path (Join-Path $roaming 'keep.txt')) -or !(Test-Path (Join-Path $local 'keep.txt'))) { throw 'Config removed without consent' }
if (!(Test-Path (Join-Path $install 'unrelated.txt'))) { throw 'Unrelated file removed' }
Write-Output 'PASS: actual deletion, completion marker, registration, data preservation, unrelated file preservation'
Install-Fixture
$lockedPath = Join-Path $install 'resources\resource.txt'
$lock = [System.IO.File]::Open($lockedPath, 'Open', 'Read', 'None')
try { $code = Uninstall-Fixture } finally { $lock.Dispose() }
if ($code -eq 0 -or (Get-Content $status -Raw) -notlike 'error:*') { throw "Locked file incorrectly succeeded: $code" }
if (!(Test-Path $reg) -or !(Test-Path (Join-Path $install 'uninstall.exe'))) { throw 'Lost uninstall entry on failure' }
if ((Get-Content $status -Raw) -notlike ('*' + $lockedPath + '*')) { throw 'Unicode failure path was lost' }
Write-Output 'PASS: locked resource fails, reports its Unicode path and retains uninstall entry'
$external = Join-Path $testRoot 'external game data'
New-Item -ItemType Directory -Path $external -Force | Out-Null
Set-Content (Join-Path $external 'save.txt') 'world data'
New-Item -ItemType Junction -Path (Join-Path $roaming 'external-link') -Target $external | Out-Null
$code = Uninstall-Fixture '/DELETE_APP_DATA'
if ($code -ne 0) { throw "Retry failed: $code $(Get-Content $status)" }
if ((Test-Path $roaming) -or (Test-Path $local)) { throw 'Selected app data remains' }
if (!(Test-Path (Join-Path $external 'save.txt'))) { throw 'External data was deleted' }
Write-Output 'PASS: retry succeeds, selected app data removed, junction target preserved'

# Access denied must behave like any other failed deletion, even when directory creation succeeds.
Install-Fixture
$principal = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$mainPath = Join-Path $install 'AxolotlUninstallFixture.exe'
& icacls $mainPath /deny ('*' + $principal + ':(D)') | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Failed to set file permissions for test' }
& icacls $install /deny ('*' + $principal + ':(DC)') | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Failed to set directory permissions for test' }
try { $code = Uninstall-Fixture } finally {
  & icacls $mainPath /remove:d ('*' + $principal) | Out-Null
  & icacls $install /remove:d ('*' + $principal) | Out-Null
}
if ($code -eq 0 -or (Get-Content $status -Raw) -notlike 'error:*') { throw 'Access denied incorrectly succeeded' }
if (!(Test-Path $reg) -or !(Test-Path $mainPath)) { throw 'Permission failure lost the installed state' }
$code = Uninstall-Fixture
if ($code -ne 0) { throw 'Permission recovery failed' }
Write-Output 'PASS: access denied fails visibly, retry works after permissions are restored'

# Update and migration callers bypass the Webview and preserve data as before.
Install-Fixture
New-Item -ItemType Directory -Path $roaming,$local -Force | Out-Null
Set-Content (Join-Path $roaming 'keep.txt') 'config'
Set-Content (Join-Path $local 'keep.txt') 'cache'
$code = Uninstall-Fixture '/UPDATE /DELETE_APP_DATA'
if ($code -ne 0 -or !(Test-Path (Join-Path $roaming 'keep.txt')) -or !(Test-Path (Join-Path $local 'keep.txt'))) { throw 'Update uninstall did not preserve data' }
Write-Output 'PASS: update uninstall preserves data even with DELETE_APP_DATA'
Install-Fixture
$code = Run-Core (Join-Path $testRoot 'core.exe') ('/S /P /STATUS_FILE="' + $status + '" _?=' + $install)
if ($code -ne 0 -or (Get-Content $status -Raw) -ne '100') { throw 'Passive migration uninstall failed' }
Write-Output 'PASS: passive migration executes the uninstall core without launching the Webview'

# Ordinary /S has NSIS's normal copy-and-relaunch lifecycle.
Install-Fixture
Set-Content $status 'pending'
$code = Run-Core (Join-Path $install 'uninstall.exe') ('/S /STATUS_FILE="' + $status + '"')
$deadline = [DateTime]::UtcNow.AddSeconds(30)
do {
  $result = Get-Content $status -Raw
  if ($result -eq '100' -or $result -like 'error:*') { break }
  Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)
if ($code -ne 0 -or $result -ne '100' -or (Test-Path $reg)) { throw 'Ordinary silent uninstall did not complete' }
Write-Output 'PASS: ordinary silent uninstall waits for real completion through its status file'

# A junction at the data root must be removed without enumerating the external target.
Install-Fixture
$code = Uninstall-Fixture '/DELETE_APP_DATA'
if ($code -ne 0) { throw 'Data cleanup setup failed' }
New-Item -ItemType Junction -Path $roaming -Target $external | Out-Null
Install-Fixture
$code = Uninstall-Fixture '/DELETE_APP_DATA'
if ($code -ne 0 -or (Test-Path $roaming) -or !(Test-Path (Join-Path $external 'save.txt'))) { throw 'Root junction cleanup was unsafe' }
Write-Output 'PASS: root junction is removed and external save data stays intact'
