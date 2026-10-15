#!/usr/bin/env bash
# Native Android build: tools/apk.sh [--install] → dist/hst.apk (NativeActivity, no gradle; the activity's few lines of
# Java, crates/hst/android/, go through javac + d8).
# HST_ABIS="arm64-v8a x86_64" adds the emulator's ABI (default: phones only, arm64-v8a).
# Needs ~/Android/Sdk (build-tools, platforms/android-36, ndk) and `rustup target add aarch64-linux-android x86_64-linux-android`.
# The disc: picked on the phone on first launch (M99), or adb push "<ISO>" /sdcard/Android/data/io.github.fizl0.hst/files/
set -euo pipefail
cd "$(dirname "$0")/.."
SDK=${ANDROID_HOME:-$HOME/Android/Sdk}
NDK=${ANDROID_NDK_HOME:-$(ls -d "$SDK"/ndk/* | sort -V | tail -1)}
BT=$(ls -d "$SDK"/build-tools/* | sort -V | tail -1)
JAR=$(ls -d "$SDK"/platforms/android-* | sort -V | tail -1)/android.jar
T=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin
out=target/apk; rm -rf "$out/lib"; mkdir -p dist
for abi in ${HST_ABIS:-arm64-v8a}; do
    case $abi in arm64-v8a) t=aarch64-linux-android ;; x86_64) t=x86_64-linux-android ;; *) echo "unknown ABI $abi"; exit 2 ;; esac
    T_=${t//-/_}; cc=$T/${t}26-clang
    env "CARGO_TARGET_${T_^^}_LINKER=$cc" "CC_$T_=$cc" "CXX_$T_=$cc++" "AR_$T_=$T/llvm-ar" \
        cargo build --release -p hst --example hst --features android --target $t
    mkdir -p "$out/lib/$abi"
    "$T/llvm-strip" -o "$out/lib/$abi/libhst.so" target/$t/release/examples/libhst.so
done
cat >"$out/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="io.github.fizl0.hst"
    android:versionCode="1" android:versionName="0.1">
  <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="36"/>
  <uses-feature android:glEsVersion="0x00030000"/>
  <uses-permission android:name="android.permission.INTERNET"/>
  <application android:label="Hot Shots Tennis" android:hasCode="true" android:extractNativeLibs="false"
      android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
    <activity android:name="io.github.fizl0.hst.HstActivity" android:exported="true" android:screenOrientation="sensorLandscape"
        android:configChanges="orientation|screenSize|screenLayout|keyboardHidden|keyboard|navigation|uiMode"
        android:resizeableActivity="true">
      <meta-data android:name="android.app.lib_name" android:value="hst"/>
      <intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter>
    </activity>
    <activity android:name="io.github.fizl0.hst.Restart" android:process=":restart" android:exported="false"
        android:theme="@android:style/Theme.NoDisplay"/>
  </application>
</manifest>
EOF
rm -rf "$out/classes" && mkdir -p "$out/classes" "$out/dex"
javac -nowarn --release 17 -classpath "$JAR" -d "$out/classes" $(find crates/hst/android -name '*.java')
"$BT/d8" --release --min-api 26 --lib "$JAR" --output "$out/dex" $(find "$out/classes" -name '*.class')
"$BT/aapt2" link -o "$out/base.apk" --manifest "$out/AndroidManifest.xml" -I "$JAR" -A crates/hst/assets
(cd "$out" && zip -q -0 -r base.apk lib && zip -q -j base.apk dex/classes.dex)
"$BT/zipalign" -f -p 4 "$out/base.apk" "$out/aligned.apk"
key=$HOME/.android/debug.keystore
[ -f "$key" ] || keytool -genkeypair -keystore "$key" -storepass android -alias androiddebugkey -keypass android \
    -keyalg RSA -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null
"$BT/apksigner" sign --ks "$key" --ks-pass pass:android --out dist/hst.apk "$out/aligned.apk"
echo "dist/hst.apk ($(du -h dist/hst.apk | cut -f1))"
[ "${1:-}" = --install ] && "$SDK/platform-tools/adb" install -r dist/hst.apk
exit 0
