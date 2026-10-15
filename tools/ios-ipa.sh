#!/usr/bin/env bash
# Unsigned IPA of hst for the user to sign (M91): release build (tools/ios-build.sh), stripped, in a
# Payload/HotShotsTennis.app with Info.plist + icon, zipped. Usage: tools/ios-ipa.sh [OUT.ipa]  (default target/HotShotsTennis.ipa)
# On the phone: the app's folder in the Files app ("On My iPhone → Hot Shots") takes the ISO, settings.txt and mods/.
set -euo pipefail
cd "$(dirname "$0")/.."
out=$(realpath -m "${1:-target/HotShotsTennis.ipa}")
HST_BUILD=$(git rev-list --count HEAD) tools/ios-build.sh build --release -p hst
stage=target/aarch64-apple-ios/ipa
app=$stage/Payload/HotShotsTennis.app
rm -rf target/aarch64-apple-ios/ipa
mkdir -p "$app"
llvm-strip -o "$app/hst" target/aarch64-apple-ios/release/hst
cp web/icon-180.png "$app/AppIcon60x60@3x.png"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/hst/Cargo.toml | head -1)
cat > "$app/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>hst</string>
	<key>CFBundleIdentifier</key><string>io.github.fizl0.hst</string>
	<key>CFBundleName</key><string>Hot Shots Tennis</string>
	<key>CFBundleDisplayName</key><string>Hot Shots</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>CFBundleVersion</key><string>$(git rev-list --count HEAD)</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleSupportedPlatforms</key><array><string>iPhoneOS</string></array>
	<key>DTPlatformName</key><string>iphoneos</string>
	<key>MinimumOSVersion</key><string>15.0</string>
	<key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
	<key>UIRequiredDeviceCapabilities</key><array><string>arm64</string><string>metal</string></array>
	<key>CFBundleIcons</key><dict><key>CFBundlePrimaryIcon</key><dict>
		<key>CFBundleIconFiles</key><array><string>AppIcon60x60</string></array>
	</dict></dict>
	<key>UILaunchScreen</key><dict/>
	<key>UIRequiresFullScreen</key><true/>
	<key>UIStatusBarHidden</key><true/>
	<key>UISupportedInterfaceOrientations</key><array>
		<string>UIInterfaceOrientationLandscapeLeft</string><string>UIInterfaceOrientationLandscapeRight</string>
	</array>
	<key>UISupportedInterfaceOrientations~ipad</key><array>
		<string>UIInterfaceOrientationLandscapeLeft</string><string>UIInterfaceOrientationLandscapeRight</string>
	</array>
	<key>UIFileSharingEnabled</key><true/>
	<key>LSSupportsOpeningDocumentsInPlace</key><true/>
	<key>NSLocalNetworkUsageDescription</key><string>Finds and joins matches on your local network.</string>
</dict>
</plist>
EOF
rm -f "$out"
(cd "$stage" && zip -qr -X "$out" Payload)
echo "$out ($(du -h "$out" | cut -f1))"
