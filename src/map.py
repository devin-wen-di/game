from matplotlib.path import Path
from matplotlib.colors import ListedColormap, BoundaryNorm
import numpy as np

# 画布尺寸与坐标范围
width = 128 * 3
height = 128 * 2
cols = width * 2
rows = height * 2

x_vals = np.linspace(-width, width, cols)
y_vals = np.linspace(-height, height, rows)
X, Y = np.meshgrid(x_vals, y_vals)

a = width / 10000
x0 = width / 3
t = np.linspace(0, 2 * np.pi, 1000)
cloud_x = width / 3 * np.cos(t) - width / 2 + width / 40 * (np.cos(10 * t) + 1)
cloud_y = height / 8 * np.sin(t) + height / 2 + height / 200 * (np.sin(10 * t) + 1)

sigmoid_y = height / (1 + np.exp(-a * (x_vals - x0))) - height + height / 2 + height / 50 * np.sin(15 / width * x_vals)

canvas = np.zeros((rows, cols), dtype=np.uint8)
canvas[Y <= sigmoid_y] = 1

polygon = Path(np.column_stack((cloud_x, cloud_y)))
points = np.column_stack((X.ravel(), Y.ravel()))
cloud_mask = polygon.contains_points(points).reshape(rows, cols)
canvas[cloud_mask] = 2

np.savetxt('cloud_scene.csv', canvas, fmt='%d', delimiter=',')
