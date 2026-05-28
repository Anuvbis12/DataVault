$NDK_BIN = "C:\Users\alama\AppData\Local\Android\Sdk\ndk\30.0.14904198\toolchains\llvm\prebuilt\windows-x86_64\bin"
$SYSROOT  = "C:\Users\alama\AppData\Local\Android\Sdk\ndk\30.0.14904198\toolchains\llvm\prebuilt\windows-x86_64\sysroot"
$env:CC_aarch64_linux_android   = "$NDK_BIN\clang.exe"
$env:CXX_aarch64_linux_android  = "$NDK_BIN\clang++.exe"
$env:AR_aarch64_linux_android   = "$NDK_BIN\llvm-ar.exe"
$env:CFLAGS_aarch64_linux_android  = "--target=aarch64-linux-android24 --sysroot=$SYSROOT"
$env:CXXFLAGS_aarch64_linux_android = "--target=aarch64-linux-android24 --sysroot=$SYSROOT"
$env:ANDROID_NDK_HOME = "C:\Users\alama\AppData\Local\Android\Sdk\ndk\30.0.14904198"
cargo check --target aarch64-linux-android 2>&1
