# Builds a signed debug APK for a Kathamo cdylib crate (NativeActivity, no Java).
#   pwsh tools/android/build_apk.ps1                       # the demo, arm64 + x86_64
#   pwsh tools/android/build_apk.ps1 -Crate my-app -Package com.me.app -Label "My App"
#   pwsh tools/android/build_apk.ps1 -Install              # also adb install + launch
#   pwsh tools/android/build_apk.ps1 -Crate kathamo-setu-remote -Package studio.kalponic.setu.remote `
#       -Label "Setu Remote" -Permission INTERNET            # phone controller (needs the network)
# Needs: Android SDK (build-tools, platforms), NDK, `cargo install cargo-ndk`,
# `rustup target add aarch64-linux-android x86_64-linux-android`.
param(
    [string]$Crate = 'kathamo-android-demo',
    [string]$Package = 'studio.kalponic.kathamo.demo',
    [string]$Label = 'Kathamo',
    [string[]]$Abi = @('arm64-v8a', 'x86_64'),
    # Android permissions, e.g. INTERNET for an app that talks to a host. Short
    # names get the android.permission. prefix.
    [string[]]$Permission = @(),
    [switch]$Install
)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\..\.."
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { 'C:\Android' }
if (-not $env:ANDROID_NDK_HOME) {
    $ndk = Get-ChildItem "$sdk\ndk" -Directory -ErrorAction SilentlyContinue | Sort-Object Name | Select-Object -Last 1
    if (-not $ndk) { throw "No NDK under $sdk\ndk. Install: sdkmanager `"ndk;27.2.12479018`"" }
    $env:ANDROID_NDK_HOME = $ndk.FullName
}
$env:ANDROID_HOME = $sdk
$bt = Get-ChildItem "$sdk\build-tools" -Directory | Sort-Object Name | Select-Object -Last 1
$platform = Get-ChildItem "$sdk\platforms" -Directory | Sort-Object { [int]($_.Name -replace '\D', '') } | Select-Object -Last 1
$lib = $Crate -replace '-', '_'
$targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "$root\target" }
$out = "$targetDir\android-apk\$Crate"
New-Item -ItemType Directory -Force "$out\stage" | Out-Null

# 1. Rust: one .so per ABI, placed as lib/<abi>/lib<name>.so.
Push-Location $root
try {
    $targets = $Abi | ForEach-Object { '-t'; $_ }
    & cargo ndk @targets -o "$out\stage\lib" build --release -p $Crate --lib
    if ($LASTEXITCODE) { throw "cargo ndk failed ($LASTEXITCODE)" }
} finally { Pop-Location }

# 2. Manifest -> binary APK skeleton (no resources, no dex).
$perms = ($Permission | ForEach-Object {
        $name = if ($_ -match '\.') { $_ } else { "android.permission.$_" }
        "    <uses-permission android:name=`"$name`" />"
    }) -join "`n"
(Get-Content "$PSScriptRoot\AndroidManifest.xml" -Raw).Replace('@PACKAGE@', $Package).Replace('@LABEL@', $Label).Replace('@LIB@', $lib).Replace('@PERMISSIONS@', $perms) |
    Set-Content -Encoding utf8 "$out\AndroidManifest.xml"
& "$($bt.FullName)\aapt2.exe" link --manifest "$out\AndroidManifest.xml" -I "$($platform.FullName)\android.jar" -o "$out\unaligned.apk"
if ($LASTEXITCODE) { throw 'aapt2 link failed' }

# 3. Add the native libraries (paths inside the zip must use forward slashes).
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::Open("$out\unaligned.apk", 'Update')
try {
    Get-ChildItem "$out\stage\lib" -Recurse -Filter *.so | ForEach-Object {
        $entry = 'lib/' + $_.Directory.Name + '/' + $_.Name
        [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $_.FullName, $entry) | Out-Null
    }
} finally { $zip.Dispose() }

# 4. Align, then sign with the standard debug key (made on first use).
$apk = "$out\$Crate.apk"
& "$($bt.FullName)\zipalign.exe" -f -p 4 "$out\unaligned.apk" $apk
if ($LASTEXITCODE) { throw 'zipalign failed' }
$ks = "$env:USERPROFILE\.android\debug.keystore"
if (-not (Test-Path $ks)) {
    & keytool -genkeypair -keystore $ks -storepass android -keypass android -alias androiddebugkey `
        -keyalg RSA -keysize 2048 -validity 10000 -dname 'CN=Android Debug,O=Android,C=US'
}
& "$($bt.FullName)\apksigner.bat" sign --ks $ks --ks-pass pass:android --key-pass pass:android $apk
if ($LASTEXITCODE) { throw 'apksigner failed' }
"APK: $apk ($([math]::Round((Get-Item $apk).Length / 1MB, 1)) MB)"

if ($Install) {
    $adb = "$sdk\platform-tools\adb.exe"
    & $adb install -r $apk
    & $adb shell am start -n "$Package/android.app.NativeActivity"
}
