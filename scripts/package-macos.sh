#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'Este empaquetado requiere macOS.' >&2
  exit 1
fi
app_version="$(node -p "require('./package.json').version")"
app="$project_dir/src-tauri/target/release/bundle/macos/XD2Figma.app"
case "${1:-}" in
  '') pnpm tauri build --bundles app -- --locked ;;
  --reuse-app)
    built_version="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")"
    [[ "$built_version" == "$app_version" ]] || { echo 'La app compilada no corresponde a esta versión.' >&2; exit 1; }
    /usr/bin/codesign --verify --deep --strict "$app"
    ;;
  *) echo 'Uso: pnpm package:mac [--reuse-app]' >&2; exit 1 ;;
esac
case "$(uname -m)" in
  arm64) app_arch=aarch64 ;;
  x86_64) app_arch=x64 ;;
  *) echo 'Arquitectura de macOS no compatible.' >&2; exit 1 ;;
esac
python="$project_dir/src-tauri/target/dmg-tools/bin/python"
if [[ ! -x "$python" ]]; then python3 -m venv "$project_dir/src-tauri/target/dmg-tools"; fi
if ! "$python" -c "import importlib.metadata as m; assert m.version('dmgbuild') == '1.6.5'" 2>/dev/null; then
  "$python" -m pip install -r scripts/dmg/requirements.txt
fi
artwork="$project_dir/src-tauri/target/dmg-artwork"
/usr/bin/swift -module-cache-path "$project_dir/src-tauri/target/swift-cache" scripts/dmg/background.swift "$artwork" "$app_version" "$app"
installer="$project_dir/XD2Figma_${app_version}_${app_arch}.dmg"
# Build to a new filename and replace the previous installer only after validation.
temporary_installer="$project_dir/src-tauri/target/XD2Figma_${app_version}_${app_arch}.dmg"
"$python" -m dmgbuild -s scripts/dmg/settings.py -D "app=$app" -D "background=$artwork/background.tiff" -D "icon=$project_dir/src-tauri/icons/icon.icns" XD2Figma "$temporary_installer"
/usr/bin/codesign --force --sign - "$temporary_installer"
/usr/bin/hdiutil verify "$temporary_installer"
mv -f "$temporary_installer" "$installer"
echo "Instalador: $installer"
echo "Vista previa del diseño: $artwork/preview.png"
