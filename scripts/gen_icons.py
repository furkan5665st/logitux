import os
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "assets")
SRC = os.path.join(ASSETS, "logitux-source.png")

def build_icon(size):
    scale = 4
    big = size * scale
    canvas = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    mask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, big - 1, big - 1), radius=int(big * 0.2), fill=255
    )
    canvas.paste(Image.new("RGBA", (big, big), (255, 255, 255, 255)), (0, 0), mask)

    logo = Image.open(SRC).convert("RGBA")
    logo = logo.crop(logo.getbbox())
    inner = int(big * 0.86)
    ratio = min(inner / logo.width, inner / logo.height)
    logo = logo.resize((int(logo.width * ratio), int(logo.height * ratio)), Image.LANCZOS)
    canvas.alpha_composite(logo, ((big - logo.width) // 2, (big - logo.height) // 2))
    return canvas.resize((size, size), Image.LANCZOS)

def main():
    build_icon(256).save(os.path.join(ASSETS, "logitux.png"), optimize=True)
    build_icon(256).save(
        os.path.join(ASSETS, "logitux.ico"),
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    with open(os.path.join(ASSETS, "logitux-64.rgba"), "wb") as f:
        f.write(build_icon(64).tobytes())
    print("icons generated")

if __name__ == "__main__":
    main()
