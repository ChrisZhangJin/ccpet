"""
Generate app icons from src/assets/pet.png for Tauri bundling.
Called automatically by `npm run gen-icons` (before each tauri build).
"""
import sys
import os

# --- Pillow check ---
try:
    from PIL import Image, ImageFilter
except ImportError:
    print("[gen-icons] ERROR: Pillow not installed. Run:  pip install Pillow")
    print("[gen-icons] Skipping icon generation (build will use existing icons).")
    sys.exit(0)  # exit 0 so build continues with old icons instead of failing

# --- Paths (relative to this script's location) ---
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
SRC = os.path.join(PROJECT_ROOT, "src", "assets", "pet.png")
OUT_DIR = os.path.join(PROJECT_ROOT, "src-tauri", "icons")

if not os.path.isfile(SRC):
    print(f"[gen-icons] WARNING: Source not found: {SRC}")
    print("[gen-icons] Skipping icon generation.")
    sys.exit(0)

# --- Load & square-crop ---
img = Image.open(SRC).convert("RGBA")
print(f"[gen-icons] Source: {os.path.basename(SRC)} ({img.size[0]}x{img.size[1]})")

w, h = img.size
if w != h:
    side = min(w, h)
    left, top = (w - side) // 2, (h - side) // 2
    img = img.crop((left, top, left + side, top + side))
    print(f"[gen-icons] Cropped to square: {img.size}")

def make_icon(src, size):
    out = src.resize((size, size), Image.LANCZOS)
    if size <= 64:
        out = out.filter(ImageFilter.SHARPEN)
    return out

# --- PNG variants ---
for filename, size in {
    "32x32.png": 32,
    "128x128.png": 128,
    "128x128@2x.png": 256,
    "icon.png": 512,
}.items():
    make_icon(img, size).save(os.path.join(OUT_DIR, filename), "PNG")
    print(f"  -> {filename} ({size}x{size})")

# --- .ico (multi-res) ---
ico_sizes = [(16,16),(32,32),(48,48),(64,64),(128,128),(256,256)]
ico_path = os.path.join(OUT_DIR, "icon.ico")
img.save(ico_path, format="ICO", sizes=ico_sizes)
print(f"  -> icon.ico ({os.path.getsize(ico_path)//1024} KB)")

# --- .icns (macOS) ---
try:
    icns_path = os.path.join(OUT_DIR, "icon.icns")
    img.save(icns_path, format="ICNS")
    print(f"  -> icon.icns ({os.path.getsize(icns_path)//1024} KB)")
except Exception:
    make_icon(img, 512).save(os.path.join(OUT_DIR, "icon.icns"), "PNG")
    print("  -> icon.icns (PNG fallback)")

print("[gen-icons] Done!")
