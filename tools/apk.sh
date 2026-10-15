#!/usr/bin/env bash
# Native Android build: tools/apk.sh [--install] → dist/hst.apk (NativeActivity, no Java/gradle).
# HST_ABIS="arm64-v8a x86_64" adds the emulator's ABI (default: phones only, arm64-v8a).
# Needs ~/Android/Sdk (build-tools, platforms/android-36, ndk) and `rustup target add aarch64-linux-android x86_64-linux-android`.
# The disc: adb push "<ISO>" /sdcard/Android/data/io.github.fizl0.hst/files/
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
# Launcher icon: the OpenTennis badge (web/logo.svg minus its black backdrop) on the court green. Adaptive foreground
# is 108dp, the badge 64dp wide (its drawn corners stay inside the 72dp mask circle); legacy ones are 48dp square/round.
bg='#1d5c3a'; res=$out/res; rm -rf "$res"; mkdir -p "$res/mipmap-anydpi-v26" "$res/values"
sed '/<rect width="100%" height="100%" fill="#000"\/>/d' web/logo.svg | rsvg-convert -w 2800 | magick - -trim +repage "$out/badge.png"
# Themed (Android 13+) monochrome layer: the launcher tints its alpha, so alpha = the badge's alpha × its brightness
# (12–42% → 0–1): the black outlines drop out and the lettering, court lines and ball stand out of the ring.
magick "$out/badge.png" \( +clone -alpha off -colorspace gray -level 12%,42% \) \( -clone 0 -alpha extract \) -delete 0 \
    -compose multiply -composite "$out/mono_a.png"
magick "$out/badge.png" -fill white -colorize 100 "$out/mono_a.png" -alpha off -compose copy-opacity -composite "$out/mono.png"
dp() { awk "BEGIN{print int($1*$s+.5)}"; }
for d in mdpi:1 hdpi:1.5 xhdpi:2 xxhdpi:3 xxxhdpi:4; do
    s=${d#*:}; dir=$res/mipmap-${d%:*}; mkdir -p "$dir"; a=$(dp 108); l=$(dp 48); r=$(awk "BEGIN{print 24*$s}")
    magick -size ${a}x$a xc:none \( "$out/badge.png" -resize "$(dp 64)x" \) -gravity center -composite "$dir/ic_launcher_foreground.png"
    magick -size ${a}x$a xc:none \( "$out/mono.png" -resize "$(dp 64)x" \) -gravity center -composite "$dir/ic_launcher_monochrome.png"
    magick -size ${l}x$l xc:none -fill "$bg" -draw "roundrectangle 0,0 $((l-1)),$((l-1)) $(dp 8),$(dp 8)" \
        \( "$out/badge.png" -resize "$(dp 44)x" \) -gravity center -composite "$dir/ic_launcher.png"
    magick -size ${l}x$l xc:none -fill "$bg" -draw "circle $r,$r $r,0" \
        \( "$out/badge.png" -resize "$(dp 40)x" \) -gravity center -composite "$dir/ic_launcher_round.png"
done
echo "<resources><color name=\"ic_launcher_background\">$bg</color></resources>" >"$res/values/ic_launcher.xml"
for n in ic_launcher ic_launcher_round; do
    echo '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
  <background android:drawable="@color/ic_launcher_background"/><foreground android:drawable="@mipmap/ic_launcher_foreground"/>
  <monochrome android:drawable="@mipmap/ic_launcher_monochrome"/>
</adaptive-icon>' >"$res/mipmap-anydpi-v26/$n.xml"
done
rm -f "$out/res.zip"; "$BT/aapt2" compile --dir "$res" -o "$out/res.zip"
cat >"$out/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="io.github.fizl0.hst"
    android:versionCode="1" android:versionName="0.1">
  <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="36"/>
  <uses-feature android:glEsVersion="0x00030000"/>
  <uses-permission android:name="android.permission.INTERNET"/>
  <application android:label="OpenTennis" android:icon="@mipmap/ic_launcher"
      android:roundIcon="@mipmap/ic_launcher_round" android:hasCode="false" android:extractNativeLibs="false"
      android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
    <activity android:name="android.app.NativeActivity" android:exported="true" android:screenOrientation="sensorLandscape"
        android:configChanges="orientation|screenSize|screenLayout|keyboardHidden|keyboard|navigation|uiMode"
        android:resizeableActivity="true">
      <meta-data android:name="android.app.lib_name" android:value="hst"/>
      <intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter>
    </activity>
  </application>
</manifest>
EOF
"$BT/aapt2" link -o "$out/base.apk" --manifest "$out/AndroidManifest.xml" -I "$JAR" -A crates/hst/assets "$out/res.zip"
(cd "$out" && zip -q -0 -r base.apk lib)
"$BT/zipalign" -f -p 4 "$out/base.apk" "$out/aligned.apk"
key=$HOME/.android/debug.keystore
[ -f "$key" ] || keytool -genkeypair -keystore "$key" -storepass android -alias androiddebugkey -keypass android \
    -keyalg RSA -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null
"$BT/apksigner" sign --ks "$key" --ks-pass pass:android --out dist/hst.apk "$out/aligned.apk"
echo "dist/hst.apk ($(du -h dist/hst.apk | cut -f1))"
[ "${1:-}" = --install ] && "$SDK/platform-tools/adb" install -r dist/hst.apk
exit 0
