Write-Host "Membangun Aegis Vault untuk Android..." -ForegroundColor Cyan

Write-Host "Kompilasi Rust libaegis_vault.so dengan cargo-ndk..." -ForegroundColor Cyan
cargo ndk -t arm64-v8a -o ./android/app/src/main/jniLibs build --lib

if ($LASTEXITCODE -ne 0) {
    Write-Host "Kompilasi Rust Gagal!" -ForegroundColor Red
    exit 1
}

Write-Host "Kompilasi APK dengan Gradle..." -ForegroundColor Cyan
Set-Location -Path "android"

if (!(Test-Path "gradlew.bat")) {
    Write-Host "==================================================" -ForegroundColor Yellow
    Write-Host "PERHATIAN: Gradle Wrapper belum terpasang." -ForegroundColor Yellow
    Write-Host "Silakan buka folder 'D:\DataVault\android' menggunakan Android Studio." -ForegroundColor Yellow
    Write-Host "Android Studio akan mengunduh Gradle secara otomatis." -ForegroundColor Yellow
    Write-Host "Setelah itu Anda bisa menekan tombol 'Run' (Segitiga Hijau) di Android Studio" -ForegroundColor Yellow
    Write-Host "Atau jalankan ulang script ini." -ForegroundColor Yellow
    Write-Host "==================================================" -ForegroundColor Yellow
    Set-Location -Path ".."
    exit
}

.\gradlew assembleDebug

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build sukses! APK ada di: android\app\build\outputs\apk\debug\app-debug.apk" -ForegroundColor Green
} else {
    Write-Host "Build gagal." -ForegroundColor Red
}

Set-Location -Path ".."
