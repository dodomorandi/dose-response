Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$PSDefaultParameterValues['*:ErrorAction']='Stop'

If ($Env:APPVEYOR_REPO_TAG == "false") {
    Write-Host "This is not a release tag. Running test builds."
    # Print all environment variables
    gci env:* | sort-object name

    cargo build --features "test windows-extra-features" --target %TARGET%
    $native_call_success = $?
    if (-not $native_call_success) {
        throw 'error making native call'
    }
    cargo build --features "test windows-extra-features" --target %TARGET% --release
    $native_call_success = $?
    if (-not $native_call_success) {
        throw 'error making native call'
    }
} Else {
    Write-Host "This is a release tag: $Env:APPVEYOR_REPO_TAG. Skipping test builds."
}
