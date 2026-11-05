view = await render_view(camera="front", width=1280, height=720, samples=16)
png = await read_image(path=view["path"])
image(png)
text(f"rendered {view['width']}x{view['height']} to {view['path']} ({round(len(png) * 3 / 4 / 1024)} KiB)")
