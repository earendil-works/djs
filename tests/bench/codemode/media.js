// Render a view, fetch the image, hand it to the host (about 13% of the bodies).
const view = await tools.render_view({ camera: "front", width: 1280, height: 720, samples: 16 });
const png = await tools.read_image({ path: view.path });
image(png);
text(`rendered ${view.width}x${view.height} to ${view.path} (${Math.round(png.length * 3 / 4 / 1024)} KiB)`);
