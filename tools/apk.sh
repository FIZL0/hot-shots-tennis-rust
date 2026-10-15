#!/usr/bin/env bash
# Native Android build: tools/apk.sh [--install] → dist/opentennis.apk (NativeActivity, no gradle; the activity's few lines of
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
# Launcher icon: the iPhone app's (web/icon.svg, the ball on the court green). Its 512 frame maps onto the 72dp the
# adaptive mask shows (the ball, 42dp, inside the 66dp safe zone); the legacy icons are the whole frame at 48dp.
bg='#1d5c3a'; res=$out/res; rm -rf "$res"; mkdir -p "$res/mipmap-anydpi-v26" "$res/values"
sed '/<rect /d' web/icon.svg | rsvg-convert -w 1024 >"$out/badge.png"
# Themed (Android 13+) monochrome layer: the launcher tints its alpha, so the ball with its seams cut out.
sed -e '/<rect /d' -e 's|<circle |<mask id="m"><rect width="512" height="512" fill="#fff"/><path d="M137 160a170 170 0 0 1 0 192M375 160a170 170 0 0 0 0 192" fill="none" stroke="#000" stroke-width="16" stroke-linecap="round"/></mask><circle mask="url(#m)" |' \
    -e '/<path d=.*stroke="#fff"/d' web/icon.svg | rsvg-convert -w 1024 >"$out/mono.png"
dp() { awk "BEGIN{print int($1*$s+.5)}"; }
for d in mdpi:1 hdpi:1.5 xhdpi:2 xxhdpi:3 xxxhdpi:4; do
    s=${d#*:}; dir=$res/mipmap-${d%:*}; mkdir -p "$dir"; a=$(dp 108); l=$(dp 48); r=$(awk "BEGIN{print 24*$s}")
    magick -size ${a}x$a xc:none \( "$out/badge.png" -resize "$(dp 72)x" \) -gravity center -composite "$dir/ic_launcher_foreground.png"
    magick -size ${a}x$a xc:none \( "$out/mono.png" -resize "$(dp 72)x" \) -gravity center -composite "$dir/ic_launcher_monochrome.png"
    magick -size ${l}x$l xc:none -fill "$bg" -draw "roundrectangle 0,0 $((l-1)),$((l-1)) $(dp 8),$(dp 8)" \
        \( "$out/badge.png" -resize "${l}x" \) -gravity center -composite "$dir/ic_launcher.png"
    magick -size ${l}x$l xc:none -fill "$bg" -draw "circle $r,$r $r,0" \
        \( "$out/badge.png" -resize "${l}x" \) -gravity center -composite "$dir/ic_launcher_round.png"
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
  <!-- B455: mDNS (LAN lobbies) needs Wi-Fi multicast (HstActivity's MulticastLock) -->
  <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE"/>
  <application android:label="OpenTennis" android:icon="@mipmap/ic_launcher"
      android:roundIcon="@mipmap/ic_launcher_round" android:hasCode="true" android:extractNativeLibs="false"
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
# a release (tools/release.sh, HST_VERSION=v1.2.3) names its version; versionCode 10203 keeps each release an upgrade
ver=${HST_VERSION:-}; ver=${ver#v}; ver=${ver%%-*}
vargs=()
[[ $ver =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]] && vargs=(--replace-version --version-name "${HST_VERSION#v}" \
    --version-code $((BASH_REMATCH[1] * 10000 + BASH_REMATCH[2] * 100 + BASH_REMATCH[3])))
"$BT/aapt2" link -o "$out/base.apk" --manifest "$out/AndroidManifest.xml" -I "$JAR" -A crates/hst/assets "${vargs[@]}" "$out/res.zip"
(cd "$out" && zip -q -0 -r base.apk lib && zip -q -j base.apk dex/classes.dex)
"$BT/zipalign" -f -p 4 "$out/base.apk" "$out/aligned.apk"
key=$HOME/.android/debug.keystore
[ -f "$key" ] || keytool -genkeypair -keystore "$key" -storepass android -alias androiddebugkey -keypass android \
    -keyalg RSA -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null
"$BT/apksigner" sign --ks "$key" --ks-pass pass:android --out dist/opentennis.apk "$out/aligned.apk"
echo "dist/opentennis.apk ($(du -h dist/opentennis.apk | cut -f1))"
[ "${1:-}" = --install ] && "$SDK/platform-tools/adb" install -r dist/opentennis.apk
exit 0
